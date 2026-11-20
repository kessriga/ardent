//! The tracked files of a project (spec `ardent-layout`, "Tracked files"): each file that the patterns of exactly one
//! enabled format match, outside git's ignore rules, the `exclude` patterns, `.ardent/`, and nested projects.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ardent_core::layout::ARDENT_DIR;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{DirEntry, WalkBuilder};

/// The name and the file patterns of one enabled format.
pub struct FormatPatterns<'a> {
    pub name: &'a str,
    pub files: &'a [String],
}

/// A tracked file: its source path, relative to the project root with `/` between parts, and the index of its format.
#[derive(Debug, PartialEq, Eq)]
pub struct TrackedFile {
    pub path: String,
    pub format: usize,
}

/// A file or directory that the walk skipped, and that the sync reports.
#[derive(Debug, PartialEq, Eq)]
pub enum Warning {
    TwoFormats { path: String, formats: Vec<String> },
    NotUtf8(PathBuf),
    NestedProject(PathBuf),
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TwoFormats { path, formats } => write!(
                f,
                "{path} matches the formats {}; Ardent tracks it under none of them",
                formats.join(" and ")
            ),
            Self::NotUtf8(path) => write!(
                f,
                "{} is not a UTF-8 path; Ardent does not track it",
                path.display()
            ),
            Self::NestedProject(dir) => write!(
                f,
                "{}/ is a nested Ardent project; Ardent does not track its files",
                dir.display()
            ),
        }
    }
}

/// The result of a walk. `errors` lists the paths that the walk could not read; the file list is then incomplete.
#[derive(Debug, Default)]
pub struct Walk {
    pub files: Vec<TrackedFile>,
    pub warnings: Vec<Warning>,
    pub errors: Vec<String>,
}

/// A pattern in `config.yaml` that is not valid gitignore syntax.
#[derive(Debug)]
pub struct PatternError(ignore::Error);

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid pattern: {}", self.0)
    }
}

impl std::error::Error for PatternError {}

/// The compiled patterns of `config.yaml`: the `files` patterns of each enabled format, and the `exclude` patterns.
pub struct Patterns<'a> {
    formats: Vec<(&'a str, Gitignore)>,
    exclude: Gitignore,
}

impl<'a> Patterns<'a> {
    /// Fails when a pattern is not valid gitignore syntax.
    pub fn new(
        root: &Path,
        formats: &[FormatPatterns<'a>],
        exclude: &[String],
    ) -> Result<Self, PatternError> {
        let formats = formats
            .iter()
            .map(|format| Ok((format.name, matcher(root, format.files)?)))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            formats,
            exclude: matcher(root, exclude)?,
        })
    }

    /// The indexes of the formats whose `files` patterns match `relative` itself, not one of its parent directories.
    fn formats_of(&self, relative: &Path) -> Vec<usize> {
        self.formats
            .iter()
            .enumerate()
            .filter(|(_, (_, files))| files.matched(relative, false).is_ignore())
            .map(|(i, _)| i)
            .collect()
    }
}

/// Lists the tracked files under `root`, sorted by path.
pub fn walk(root: &Path, patterns: &Patterns) -> Walk {
    let filter = Filter {
        root: root.to_path_buf(),
        exclude: patterns.exclude.clone(),
        nested_projects: Arc::new(Mutex::new(Vec::new())),
    };
    let nested_projects = Arc::clone(&filter.nested_projects);
    let mut walk = Walk::default();
    for entry in walker(root, filter) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                walk.errors.push(err.to_string());
                continue;
            }
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            continue;
        };
        let matching = patterns.formats_of(relative);
        let Some(path) = source_path(relative) else {
            if !matching.is_empty() {
                walk.warnings.push(Warning::NotUtf8(relative.to_path_buf()));
            }
            continue;
        };
        match matching[..] {
            [] => {}
            [format] => walk.files.push(TrackedFile { path, format }),
            _ => walk.warnings.push(Warning::TwoFormats {
                path,
                formats: matching
                    .iter()
                    .map(|&i| patterns.formats[i].0.to_string())
                    .collect(),
            }),
        }
    }
    if let Ok(mut nested) = nested_projects.lock() {
        walk.warnings
            .extend(nested.drain(..).map(Warning::NestedProject));
    }
    walk.files.sort_by(|a, b| a.path.cmp(&b.path));
    walk
}

/// A walker with git's ignore rules. Inside a git repository, it also reads the `.gitignore` files above the root up to
/// the repository root, and applies no outer rule inside a nested repository, as git does. Outside a git repository,
/// it reads only the `.gitignore` files of the project. Hidden files count; `.ignore` files and the global excludes
/// file of git do not.
fn walker(root: &Path, filter: Filter) -> ignore::Walk {
    let in_git_repository = root.ancestors().any(|dir| dir.join(".git").exists());
    WalkBuilder::new(root)
        .hidden(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(true)
        .git_exclude(true)
        .require_git(in_git_repository)
        .parents(in_git_repository)
        .follow_links(false)
        .filter_entry(move |entry| filter.keep(entry))
        .build()
}

/// The entries that the walk does not enter: `.git`, the project's `.ardent/`, excluded paths, and nested projects,
/// which it records. Excluded directories are never read, so an unreadable or nested project inside one is no error.
struct Filter {
    root: PathBuf,
    exclude: Gitignore,
    nested_projects: Arc<Mutex<Vec<PathBuf>>>,
}

impl Filter {
    fn keep(&self, entry: &DirEntry) -> bool {
        if entry.depth() == 0 {
            return true;
        }
        let name = entry.file_name();
        if name == ".git" || (name == ARDENT_DIR && entry.depth() == 1) {
            return false;
        }
        let Ok(relative) = entry.path().strip_prefix(&self.root) else {
            return true;
        };
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        if self.exclude.matched(relative, is_dir).is_ignore() {
            return false;
        }
        if !is_dir || !entry.path().join(ARDENT_DIR).is_dir() {
            return true;
        }
        if let Ok(mut nested) = self.nested_projects.lock() {
            nested.push(relative.to_path_buf());
        }
        false
    }
}

fn matcher(root: &Path, patterns: &[String]) -> Result<Gitignore, PatternError> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        builder.add_line(None, pattern).map_err(PatternError)?;
    }
    builder.build().map_err(PatternError)
}

/// The source path of `relative` with `/` between its parts on every platform, or `None` when it is not UTF-8.
fn source_path(relative: &Path) -> Option<String> {
    let parts: Option<Vec<&str>> = relative
        .components()
        .map(|part| part.as_os_str().to_str())
        .collect();
    parts.map(|parts| parts.join("/"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;

    use super::*;

    fn write(root: &Path, path: &str) -> io::Result<()> {
        let path = root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, "# Title\n")
    }

    fn patterns(patterns: &[&str]) -> Vec<String> {
        patterns.iter().map(|p| p.to_string()).collect()
    }

    fn tracked(root: &Path, exclude: &[&str]) -> Result<Walk, Box<dyn std::error::Error>> {
        let markdown = patterns(&["*.md", "*.markdown"]);
        let formats = [FormatPatterns {
            name: "markdown",
            files: &markdown,
        }];
        Ok(walk(
            root,
            &Patterns::new(root, &formats, &patterns(exclude))?,
        ))
    }

    fn paths(walk: &Walk) -> Vec<&str> {
        walk.files.iter().map(|file| file.path.as_str()).collect()
    }

    #[test]
    fn tracks_matching_files_sorted_by_path() -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        for path in [
            "README.md",
            "docs/guide/intro.md",
            "docs/a.markdown",
            ".github/CONTRIBUTING.md",
            "src/main.rs",
        ] {
            write(project.path(), path)?;
        }
        let walk = tracked(project.path(), &[])?;
        assert_eq!(
            paths(&walk),
            [
                ".github/CONTRIBUTING.md",
                "README.md",
                "docs/a.markdown",
                "docs/guide/intro.md"
            ]
        );
        assert!(walk.warnings.is_empty() && walk.errors.is_empty());
        Ok(())
    }

    #[test]
    fn skips_files_that_git_ignores() -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        fs::write(project.path().join(".gitignore"), "target/\n")?;
        write(project.path(), "target/doc/notes.md")?;
        write(project.path(), "notes.md")?;
        assert_eq!(paths(&tracked(project.path(), &[])?), ["notes.md"]);
        Ok(())
    }

    #[test]
    fn reads_gitignore_files_above_the_root_only_inside_a_git_repository()
    -> Result<(), Box<dyn std::error::Error>> {
        let outer = tempfile::tempdir()?;
        let root = outer.path().join("project");
        fs::write(outer.path().join(".gitignore"), "*.md\n")?;
        write(&root, "README.md")?;
        assert_eq!(paths(&tracked(&root, &[])?), ["README.md"]);
        fs::create_dir(outer.path().join(".git"))?;
        assert!(tracked(&root, &[])?.files.is_empty());
        Ok(())
    }

    #[test]
    fn ignores_gitignore_files_above_the_git_repository() -> Result<(), Box<dyn std::error::Error>>
    {
        let outer = tempfile::tempdir()?;
        let repo = outer.path().join("repo");
        fs::write(outer.path().join(".gitignore"), "*.md\n")?;
        fs::create_dir_all(repo.join(".git"))?;
        write(&repo, "README.md")?;
        assert_eq!(paths(&tracked(&repo, &[])?), ["README.md"]);
        Ok(())
    }

    #[test]
    fn applies_no_outer_gitignore_inside_a_nested_git_repository()
    -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        fs::create_dir_all(project.path().join(".git"))?;
        fs::create_dir_all(project.path().join("lib/.git"))?;
        fs::write(project.path().join(".gitignore"), "notes.md\n")?;
        write(project.path(), "notes.md")?;
        write(project.path(), "lib/notes.md")?;
        assert_eq!(paths(&tracked(project.path(), &[])?), ["lib/notes.md"]);
        Ok(())
    }

    #[test]
    fn matches_format_patterns_against_the_file_path_only() -> Result<(), Box<dyn std::error::Error>>
    {
        let project = tempfile::tempdir()?;
        write(project.path(), "notes.md/image.png")?;
        write(project.path(), "README.md")?;
        assert_eq!(paths(&tracked(project.path(), &[])?), ["README.md"]);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn does_not_enter_excluded_directories() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let project = tempfile::tempdir()?;
        fs::create_dir_all(project.path().join("vendor/lib/.ardent"))?;
        write(project.path(), "vendor/lib/README.md")?;
        let secret = project.path().join("vendor/secret");
        fs::create_dir(&secret)?;
        write(project.path(), "README.md")?;
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o000))?;
        let walk = tracked(project.path(), &["vendor/"]);
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o755))?;
        let walk = walk?;
        assert_eq!(paths(&walk), ["README.md"]);
        assert!(walk.warnings.is_empty(), "{:?}", walk.warnings);
        assert!(walk.errors.is_empty(), "{:?}", walk.errors);
        Ok(())
    }

    #[test]
    fn skips_excluded_files() -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        write(project.path(), "vendor/lib/README.md")?;
        write(project.path(), "README.md")?;
        assert_eq!(
            paths(&tracked(project.path(), &["vendor/"])?),
            ["README.md"]
        );
        Ok(())
    }

    #[test]
    fn reports_a_file_that_two_formats_match() -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        write(project.path(), "notes.md")?;
        let (markdown, notes) = (patterns(&["*.md"]), patterns(&["notes.*"]));
        let formats = [
            FormatPatterns {
                name: "markdown",
                files: &markdown,
            },
            FormatPatterns {
                name: "notes",
                files: &notes,
            },
        ];
        let walk = walk(
            project.path(),
            &Patterns::new(project.path(), &formats, &[])?,
        );
        assert!(walk.files.is_empty());
        assert_eq!(
            walk.warnings,
            [Warning::TwoFormats {
                path: "notes.md".to_string(),
                formats: patterns(&["markdown", "notes"]),
            }]
        );
        Ok(())
    }

    #[test]
    fn skips_and_reports_nested_projects() -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        fs::create_dir_all(project.path().join("tools/.ardent"))?;
        write(project.path(), "tools/README.md")?;
        write(project.path(), "README.md")?;
        let walk = tracked(project.path(), &[])?;
        assert_eq!(paths(&walk), ["README.md"]);
        assert_eq!(
            walk.warnings,
            [Warning::NestedProject(PathBuf::from("tools"))]
        );
        Ok(())
    }

    #[test]
    fn never_tracks_files_in_ardent_or_git() -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        write(project.path(), ".ardent/notes.md")?;
        write(project.path(), ".git/notes.md")?;
        assert!(tracked(project.path(), &[])?.files.is_empty());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn skips_symbolic_links() -> Result<(), Box<dyn std::error::Error>> {
        let project = tempfile::tempdir()?;
        write(project.path(), "README.md")?;
        std::os::unix::fs::symlink(
            project.path().join("README.md"),
            project.path().join("LINK.md"),
        )?;
        assert_eq!(paths(&tracked(project.path(), &[])?), ["README.md"]);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn reports_a_path_that_is_not_utf8() -> Result<(), Box<dyn std::error::Error>> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let project = tempfile::tempdir()?;
        let name = OsStr::from_bytes(b"caf\xe9.md");
        fs::write(project.path().join(name), "# Title\n")?;
        let walk = tracked(project.path(), &[])?;
        assert!(walk.files.is_empty());
        assert_eq!(walk.warnings, [Warning::NotUtf8(PathBuf::from(name))]);
        Ok(())
    }

    #[test]
    fn rejects_an_invalid_pattern() -> io::Result<()> {
        let project = tempfile::tempdir()?;
        assert!(Patterns::new(project.path(), &[], &patterns(&["[z-a]"])).is_err());
        Ok(())
    }
}
