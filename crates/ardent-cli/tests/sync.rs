//! Runs `ardent sync` in temporary projects and checks the structure files and the output (specs `sync` and
//! `ardent-layout`).

mod common;

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime};

use common::{Tree, snapshot, temp_dir};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The Markdown example of the spec `ardent-schemas`, and the structure file the spec gives for it.
const SPEC_MD: &str = "Ardent derives an outline from each source file.

## Sync rules

Every command starts with a sync.

### Renames

A rename keeps the fingerprint.
It also keeps every link.

## Example

A short example.
";
const SPEC_STRUCTURE: &str = "schema: 1
stamp:
  size: 201
  mtime: 2026-11-20T08:15:42.123456789Z
  hash: 3818c7c62f776addba39b4a76689ed32986e9e765a465f05d2131650b1c92203
  format: markdown
  format_version: 0.1.0
  format_hash: f44946daaae00ba28ceea81715b037f6b691458a901d5ac95a012af336cc5831
nodes:
  - path: docs/spec.md
    kind: file
    range: 0..201
    fingerprint: 64d285e69608de32
  - path: docs/spec.md#paragraph
    kind: paragraph
    range: 0..49
    fingerprint: 986a9a5665640a09
  - path: docs/spec.md#Sync rules
    kind: heading
    range: 50..172
    fingerprint: 6afe874c4163626f
  - path: docs/spec.md#Sync rules/paragraph
    kind: paragraph
    range: 65..99
    fingerprint: c066d35079ec2897
  - path: docs/spec.md#Sync rules/Renames
    kind: heading
    range: 100..172
    fingerprint: f2009c466ed1d13a
  - path: docs/spec.md#Sync rules/Renames/paragraph
    kind: paragraph
    range: 113..171
    fingerprint: 0b019916628c0b2b
  - path: docs/spec.md#Example
    kind: heading
    range: 172..201
    fingerprint: 5547ffff36732ee1
  - path: docs/spec.md#Example/paragraph
    kind: paragraph
    range: 184..201
    fingerprint: 7b8f8a6fa531fab0
";

/// A project made by `ardent init`, with `README.md` and `docs/spec.md`.
struct Project {
    dir: TempDir,
}

impl Project {
    fn new() -> io::Result<Self> {
        let project = Self { dir: temp_dir()? };
        let output = project.ardent_in(project.root(), "init")?;
        if !output.status.success() {
            return Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        project.write("README.md", "# Readme\n\nRead me.\n")?;
        project.write("docs/spec.md", SPEC_MD)?;
        Ok(project)
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    fn write(&self, relative: &str, contents: &str) -> io::Result<()> {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }

    fn ardent_in(&self, dir: &Path, command: &str) -> io::Result<Output> {
        Command::new(env!("CARGO_BIN_EXE_ardent"))
            .arg(command)
            .current_dir(dir)
            .output()
    }

    fn sync(&self) -> io::Result<Run> {
        self.ardent_in(self.root(), "sync").map(Run)
    }

    fn structure_path(&self, source_path: &str) -> PathBuf {
        self.path(&format!(".ardent/cache/files/{source_path}/structure.yaml"))
    }

    fn structure(&self, source_path: &str) -> io::Result<String> {
        fs::read_to_string(self.structure_path(source_path))
    }

    fn edit_structure(&self, source_path: &str, from: &str, to: &str) -> io::Result<()> {
        let text = self.structure(source_path)?;
        fs::write(self.structure_path(source_path), text.replace(from, to))
    }

    /// The tree without `.ardent/`.
    fn sources(&self) -> io::Result<Tree> {
        let mut tree = snapshot(self.root())?;
        tree.retain(|path, _| !path.starts_with(".ardent"));
        Ok(tree)
    }
}

struct Run(Output);

impl Run {
    fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.0.stdout).into_owned()
    }

    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.0.stderr).into_owned()
    }

    fn code(&self) -> Option<i32> {
        self.0.status.code()
    }

    fn assert_success(&self) -> &Self {
        assert_eq!(self.code(), Some(0), "stderr: {}", self.stderr());
        self
    }
}

/// `text` with the example values of `mtime` and `format_hash` blanked out.
fn without_example_values(text: &str) -> String {
    text.lines()
        .map(|line| match line.split_once(':') {
            Some((key @ ("  mtime" | "  format_hash"), _)) => format!("{key}: …"),
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn first_sync_writes_a_structure_file_per_tracked_file() -> TestResult {
    let project = Project::new()?;
    let run = project.sync()?;
    run.assert_success();
    assert!(project.structure_path("README.md").is_file());
    assert!(project.structure_path("docs/spec.md").is_file());
    assert!(
        run.stdout().contains("Synced 2 tracked files: 2 scanned"),
        "{}",
        run.stdout()
    );
    Ok(())
}

#[test]
fn writes_the_markdown_example_of_the_spec() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    assert_eq!(
        without_example_values(&project.structure("docs/spec.md")?),
        without_example_values(SPEC_STRUCTURE)
    );
    Ok(())
}

#[test]
fn second_sync_scans_nothing_and_writes_nothing() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    let before = snapshot(&project.path(".ardent/cache/files"))?;
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("0 scanned"), "{}", run.stdout());
    assert_eq!(snapshot(&project.path(".ardent/cache/files"))?, before);
    Ok(())
}

#[test]
fn touched_file_gets_a_new_mtime_and_keeps_its_nodes() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    let before = project.structure("README.md")?;
    let later = SystemTime::now() + Duration::from_secs(3600);
    fs::File::options()
        .write(true)
        .open(project.path("README.md"))?
        .set_modified(later)?;
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("0 scanned"), "{}", run.stdout());
    let after = project.structure("README.md")?;
    assert_ne!(after, before);
    assert_eq!(
        without_example_values(&after),
        without_example_values(&before)
    );
    Ok(())
}

#[test]
fn edited_file_is_scanned_again() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    project.write("README.md", "# Readme\n\nRead me.\n\n## Install\n")?;
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("1 scanned"), "{}", run.stdout());
    let structure = project.structure("README.md")?;
    assert!(
        structure.contains("path: README.md#Readme/Install"),
        "{structure}"
    );
    Ok(())
}

#[test]
fn structure_from_an_older_plugin_version_is_rebuilt() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    project.edit_structure(
        "README.md",
        "format_version: 0.1.0",
        "format_version: 0.0.9",
    )?;
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("1 scanned"), "{}", run.stdout());
    assert!(
        project
            .structure("README.md")?
            .contains("format_version: 0.1.0")
    );
    Ok(())
}

#[test]
fn broken_or_unknown_structure_files_are_rebuilt() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    let good = project.structure("README.md")?;
    fs::write(project.structure_path("README.md"), "nodes: [")?;
    project.edit_structure("docs/spec.md", "schema: 1", "schema: 7")?;
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("2 scanned"), "{}", run.stdout());
    assert_eq!(
        without_example_values(&project.structure("README.md")?),
        without_example_values(&good)
    );
    assert!(
        project
            .structure("docs/spec.md")?
            .starts_with("schema: 1\n")
    );
    Ok(())
}

#[test]
fn deleted_source_file_loses_its_structure() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    fs::remove_file(project.path("docs/spec.md"))?;
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("1 removed"), "{}", run.stdout());
    assert!(!project.path(".ardent/cache/files/docs").exists());
    assert!(project.structure_path("README.md").is_file());
    Ok(())
}

#[cfg(unix)]
#[test]
fn unreadable_directory_keeps_the_structure_files() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new()?;
    project.sync()?.assert_success();
    let docs = project.path("docs");
    fs::set_permissions(&docs, fs::Permissions::from_mode(0o000))?;
    let run = project.sync();
    fs::set_permissions(&docs, fs::Permissions::from_mode(0o755))?;
    let run = run?;
    assert_eq!(run.code(), Some(1), "stderr: {}", run.stderr());
    assert!(run.stderr().contains("docs"), "{}", run.stderr());
    assert!(project.structure_path("docs/spec.md").is_file());
    Ok(())
}

#[cfg(unix)]
#[test]
fn unreadable_source_file_does_not_stop_the_others() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new()?;
    let readme = project.path("README.md");
    fs::set_permissions(&readme, fs::Permissions::from_mode(0o000))?;
    let run = project.sync();
    fs::set_permissions(&readme, fs::Permissions::from_mode(0o644))?;
    let run = run?;
    assert_eq!(run.code(), Some(1), "stderr: {}", run.stderr());
    assert!(
        run.stderr().contains("ardent: README.md:"),
        "{}",
        run.stderr()
    );
    assert!(project.structure_path("docs/spec.md").is_file());
    assert!(!project.structure_path("README.md").exists());
    Ok(())
}

#[test]
fn format_without_a_plugin_gives_a_warning() -> TestResult {
    let project = Project::new()?;
    project.write("src/main.rs", "fn main() {}\n")?;
    let run = project.sync()?;
    run.assert_success();
    assert!(
        run.stderr()
            .contains("warning: the format `rust` has no plugin"),
        "{}",
        run.stderr()
    );
    assert!(!project.structure_path("src/main.rs").exists());
    Ok(())
}

#[test]
fn typo_in_the_configuration_fails_and_writes_nothing() -> TestResult {
    let project = Project::new()?;
    let config = fs::read_to_string(project.path(".ardent/config.yaml"))?;
    fs::write(
        project.path(".ardent/config.yaml"),
        config.replace("exclude:", "exlude:"),
    )?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1));
    assert!(run.stderr().contains("config.yaml"), "{}", run.stderr());
    assert!(run.stderr().contains("exlude"), "{}", run.stderr());
    assert!(!project.path(".ardent/cache").exists());
    Ok(())
}

#[test]
fn configuration_without_schema_fails() -> TestResult {
    let project = Project::new()?;
    let config = fs::read_to_string(project.path(".ardent/config.yaml"))?;
    fs::write(
        project.path(".ardent/config.yaml"),
        config.replace("schema: 1\n", ""),
    )?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1));
    assert!(run.stderr().contains("config.yaml"), "{}", run.stderr());
    assert!(run.stderr().contains("no `schema` key"), "{}", run.stderr());
    Ok(())
}

#[test]
fn sync_changes_no_file_outside_ardent() -> TestResult {
    let project = Project::new()?;
    project.write("src/main.rs", "fn main() {}\n")?;
    let before = project.sources()?;
    project.sync()?.assert_success();
    assert_eq!(project.sources()?, before);
    Ok(())
}

#[test]
fn sync_in_a_subdirectory_uses_the_project_root() -> TestResult {
    let project = Project::new()?;
    let run = Run(project.ardent_in(&project.path("docs"), "sync")?);
    run.assert_success();
    assert!(project.structure_path("docs/spec.md").is_file());
    assert!(!project.path("docs/.ardent").exists());
    Ok(())
}

#[test]
fn sync_outside_a_project_fails() -> TestResult {
    let dir = temp_dir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_ardent"))
        .arg("sync")
        .current_dir(dir.path())
        .output()?;
    let run = Run(output);
    assert_eq!(run.code(), Some(1));
    assert!(
        run.stderr().contains("is not inside an Ardent project"),
        "{}",
        run.stderr()
    );
    assert!(snapshot(dir.path())?.is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn directory_reached_through_a_symbolic_link_is_not_in_the_project() -> TestResult {
    use std::os::unix::fs as unix_fs;

    let project = Project::new()?;
    let outside = temp_dir()?;
    fs::create_dir(outside.path().join("docs"))?;
    unix_fs::symlink(outside.path().join("docs"), project.path("linked"))?;
    let run = Run(project.ardent_in(&project.path("linked"), "sync")?);
    assert_eq!(run.code(), Some(1));
    assert!(
        run.stderr().contains("is not inside an Ardent project"),
        "{}",
        run.stderr()
    );
    Ok(())
}

#[test]
fn nested_project_is_skipped_and_reported() -> TestResult {
    let project = Project::new()?;
    fs::create_dir_all(project.path("tools/.ardent"))?;
    project.write("tools/README.md", "# Tools\n")?;
    let run = project.sync()?;
    run.assert_success();
    assert!(
        run.stderr()
            .contains("warning: tools/ is a nested Ardent project"),
        "{}",
        run.stderr()
    );
    assert!(!project.structure_path("tools/README.md").exists());
    Ok(())
}

#[test]
fn same_size_edit_within_one_clock_tick_is_scanned() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    let readme = project.path("README.md");
    let mtime = fs::metadata(&readme)?.modified()?;
    project.write("README.md", "# Readme\n\nRead it.\n")?;
    // A coarse clock gives the edit, and the structure file written just before it, the mtime of the old content.
    for path in [readme, project.structure_path("README.md")] {
        fs::File::options()
            .write(true)
            .open(path)?
            .set_modified(mtime)?;
    }
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("1 scanned"), "{}", run.stdout());
    Ok(())
}

#[test]
fn renamed_file_gets_a_structure_file_under_its_new_name() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    fs::rename(project.path("README.md"), project.path("Readme.md"))?;
    project.sync()?.assert_success();
    let names: Vec<String> = fs::read_dir(project.path(".ardent/cache/files"))?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<io::Result<_>>()?;
    assert!(
        names.contains(&"Readme.md".to_string()) && !names.contains(&"README.md".to_string()),
        "{names:?}"
    );
    assert!(
        project
            .structure("Readme.md")?
            .contains("path: Readme.md\n")
    );
    Ok(())
}

#[test]
fn invalid_pattern_in_the_configuration_fails_and_writes_nothing() -> TestResult {
    let project = Project::new()?;
    let config = fs::read_to_string(project.path(".ardent/config.yaml"))?;
    fs::write(
        project.path(".ardent/config.yaml"),
        config.replace("exclude: []", r#"exclude: ["[z-a]"]"#),
    )?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1));
    assert!(run.stderr().contains("config.yaml"), "{}", run.stderr());
    assert!(run.stderr().contains("invalid pattern"), "{}", run.stderr());
    assert!(!project.path(".ardent/cache").exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_in_the_cache_stops_the_sync() -> TestResult {
    use std::os::unix::fs as unix_fs;

    let project = Project::new()?;
    let outside = temp_dir()?;
    fs::create_dir(outside.path().join("empty"))?;
    fs::create_dir(project.path(".ardent/cache"))?;
    unix_fs::symlink(outside.path(), project.path(".ardent/cache/files"))?;
    let before = snapshot(outside.path())?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1), "stderr: {}", run.stderr());
    assert!(run.stderr().contains("symlink"), "{}", run.stderr());
    assert_eq!(
        run.stderr().matches(".ardent/cache/files").count(),
        1,
        "{}",
        run.stderr()
    );
    assert_eq!(snapshot(outside.path())?, before);
    Ok(())
}
