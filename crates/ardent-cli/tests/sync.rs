//! Runs `ardent sync` in temporary projects and checks the structure files and the output (specs `sync` and
//! `ardent-layout`).

mod common;

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime};

use common::{Tree, run_init, snapshot, temp_dir};
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
const SPEC_STRUCTURE: &str = "schema: 2
stamp:
  size: 201
  mtime: 2026-11-20T08:15:42.123456789Z
  hash: 3818c7c62f776addba39b4a76689ed32986e9e765a465f05d2131650b1c92203
  format: markdown
  format_version: 0.1.0
  format_hash: f44946daaae00ba28ceea81715b037f6b691458a901d5ac95a012af336cc5831
nodes:
  - path: docs/spec.md
    label: docs/spec.md
    kind: file
    range: 0..201
    fingerprint: 64d285e69608de32
  - path: docs/spec.md#paragraph.1
    label: paragraph
    kind: paragraph
    range: 0..49
    fingerprint: 986a9a5665640a09
  - path: docs/spec.md#Sync rules
    label: Sync rules
    kind: heading
    range: 50..172
    fingerprint: 6afe874c4163626f
  - path: docs/spec.md#Sync rules/paragraph.1
    label: paragraph
    kind: paragraph
    range: 65..99
    fingerprint: c066d35079ec2897
  - path: docs/spec.md#Sync rules/Renames
    label: Renames
    kind: heading
    range: 100..172
    fingerprint: f2009c466ed1d13a
  - path: docs/spec.md#Sync rules/Renames/paragraph.1
    label: paragraph
    kind: paragraph
    range: 113..171
    fingerprint: 0b019916628c0b2b
  - path: docs/spec.md#Example
    label: Example
    kind: heading
    range: 172..201
    fingerprint: 5547ffff36732ee1
  - path: docs/spec.md#Example/paragraph.1
    label: paragraph
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
        let output = run_init(project.root(), &["--plugins", "markdown,rust"])?;
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
    project.edit_structure("docs/spec.md", "schema: 2", "schema: 1")?;
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
            .starts_with("schema: 2\n")
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
fn sync_warns_when_configured_packages_are_missing() -> TestResult {
    let project = Project::new()?;
    fs::remove_dir_all(project.path(".ardent/plugins"))?;
    let run = project.sync()?;
    run.assert_success();
    for name in ["markdown", "rust"] {
        assert!(
            run.stderr()
                .contains(&format!("the format `{name}` has no plugin"))
        );
    }
    assert!(run.stdout().contains("0 tracked files: 0 scanned"));
    Ok(())
}

#[test]
fn failed_verification_preserves_structures_and_continues_sync() -> TestResult {
    let project = Project::new()?;
    project.write("lib.rs", "fn hello() {}")?;
    project.write("legacy.rs", "fn hello() {}")?;
    project.sync()?.assert_success();
    let rust = project.structure("lib.rs")?;
    // The old schema has the same stamp, but no display labels in its nodes.
    let legacy: String = project
        .structure("legacy.rs")?
        .replacen("schema: 2", "schema: 1", 1)
        .split_inclusive('\n')
        .filter(|line| !line.starts_with("    label: "))
        .collect();
    fs::write(project.structure_path("legacy.rs"), &legacy)?;
    let stale = project.structure("README.md")?;
    fs::remove_file(project.path("README.md"))?;
    project.write("docs/spec.md", "# Revised\n")?;
    project.write(
        ".ardent/config.yaml",
        "schema: 1\nformats:\n  rust: {}\n  markdown: {files: ['*.md', '*.rs']}\n",
    )?;
    let manifest = project.path(".ardent/plugins/rust/plugin.yaml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)?.replace("*.rs", "*.txt"),
    )?;

    let run = project.sync()?;
    assert_eq!(run.code(), Some(1), "{}", run.stderr());
    assert!(run.stderr().contains("rust"));
    assert_eq!(project.structure("lib.rs")?, rust);
    assert_eq!(project.structure("legacy.rs")?, legacy);
    assert_eq!(project.structure("README.md")?, stale);
    assert!(
        project
            .structure("docs/spec.md")?
            .contains("docs/spec.md#Revised")
    );
    Ok(())
}

#[test]
fn invalid_disabled_manifest_still_prunes_deleted_files() -> TestResult {
    let project = Project::new()?;
    project.sync()?.assert_success();
    fs::remove_file(project.path("README.md"))?;
    project.write(".ardent/plugins/unused/plugin.yaml", "schema: 2\n")?;

    let run = project.sync()?;
    assert_eq!(run.code(), Some(1));
    assert!(run.stderr().contains("unused"), "{}", run.stderr());
    assert!(run.stdout().contains("1 removed"), "{}", run.stdout());
    assert!(!project.structure_path("README.md").exists());
    assert!(project.structure_path("docs/spec.md").is_file());
    Ok(())
}

#[test]
fn mis_cased_package_preserves_its_structure_when_another_format_matches() -> TestResult {
    let project = Project::new()?;
    project.write("lib.rs", "fn hello() {}\n")?;
    project.sync()?.assert_success();
    let original = project.structure("lib.rs")?;
    fs::rename(
        project.path(".ardent/plugins/rust"),
        project.path(".ardent/plugins/Rust"),
    )?;
    project.write(
        ".ardent/config.yaml",
        "schema: 1\nformats:\n  rust: {}\n  markdown: {files: ['*.md', '*.rs']}\n",
    )?;

    let run = project.sync()?;
    assert_eq!(run.code(), Some(1));
    assert!(
        run.stderr().contains("expected spelling"),
        "{}",
        run.stderr()
    );
    assert_eq!(project.structure("lib.rs")?, original);
    Ok(())
}

#[test]
fn same_version_component_replacement_invalidates_stamps() -> TestResult {
    use ardent_core::structure;
    use std::io::Write;

    let project = Project::new()?;
    project.sync()?.assert_success();
    let package = project.path(".ardent/plugins/markdown/format.wasm");
    // An empty custom section changes the component bytes while retaining its exports.
    fs::OpenOptions::new()
        .append(true)
        .open(&package)?
        .write_all(&[0, 1, 0])?;
    let hash = structure::content_hash(&fs::read(package)?);
    let run = project.sync()?;
    run.assert_success();
    assert!(run.stdout().contains("2 scanned"));
    assert!(
        project
            .structure("README.md")?
            .contains(&format!("format_hash: {hash}"))
    );

    project.edit_structure(
        "README.md",
        &hash,
        "f44946daaae00ba28ceea81715b037f6b691458a901d5ac95a012af336cc5831",
    )?;
    assert!(
        project
            .sync()?
            .assert_success()
            .stdout()
            .contains("1 scanned")
    );
    Ok(())
}

#[test]
fn oversized_source_preserves_its_structure_and_the_next_file_scans() -> TestResult {
    let project = Project::new()?;
    let first = "a.rs";
    project.write(first, "fn first() {}")?;
    project.sync()?.assert_success();
    let last_good = project.structure(first)?;
    fs::File::options()
        .write(true)
        .open(project.path(first))?
        .set_len(ardent_format::MAX_SOURCE_BYTES as u64 + 1)?;
    project.write("b.rs", "fn next() {}")?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1));
    assert!(
        run.stderr().contains("a.rs: source exceeds"),
        "{}",
        run.stderr()
    );
    assert_eq!(project.structure(first)?, last_good);
    assert!(project.structure("b.rs")?.contains("b.rs#fn next"));
    Ok(())
}

#[test]
fn scan_failures_keep_stamps_and_allow_the_next_file_to_run() -> TestResult {
    let project = Project::new()?;
    project.write(
        ".ardent/config.yaml",
        "schema: 1\nformats: {probe: {}, markdown: {}}",
    )?;
    project.write(
        ".ardent/plugins/probe/plugin.yaml",
        "schema: 1\nname: probe\nversion: 0.1.0\nkind: format\ncomponent: format.wasm\nfiles: ['*.probe']",
    )?;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/components/test-scanner.wasm");
    fs::copy(fixture, project.path(".ardent/plugins/probe/format.wasm"))?;
    for name in ["a.probe", "b.probe"] {
        project.write(name, "good")?;
    }
    project.sync()?.assert_success();
    let a = project.structure("a.probe")?;
    let b = project.structure("b.probe")?;
    project.write("a.probe", "trap")?;
    project.write("b.probe", "invalid")?;
    project.write("c.probe", "good")?;
    project.write("new.md", "# New\n")?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1), "{}", run.stderr());
    for name in ["a.probe", "b.probe"] {
        assert!(run.stderr().contains(name), "{}", run.stderr());
    }
    assert_eq!(run.stderr().lines().count(), 2, "{}", run.stderr());
    assert!(run.stderr().contains("unreachable"), "{}", run.stderr());
    assert_eq!(project.structure("a.probe")?, a);
    assert_eq!(project.structure("b.probe")?, b);
    assert!(project.structure("c.probe")?.contains("c.probe#text"));
    assert!(project.structure("new.md")?.contains("new.md#New"));
    Ok(())
}

#[test]
fn rust_2024_safe_declaration_preserves_the_previous_structure() -> TestResult {
    let project = Project::new()?;
    project.write("ffi.rs", "unsafe extern \"C\" { pub fn f(); }")?;
    project.sync()?.assert_success();
    let previous = project.structure("ffi.rs")?;
    project.write("ffi.rs", "unsafe extern \"C\" { pub safe fn f(); }")?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1), "{}", run.stderr());
    assert!(
        run.stderr()
            .contains("ffi.rs: syntax error at line 1, column 25"),
        "{}",
        run.stderr()
    );
    assert_eq!(project.structure("ffi.rs")?, previous);
    Ok(())
}

#[test]
fn format_without_a_plugin_gives_a_warning() -> TestResult {
    let project = Project::new()?;
    let config = project.path(".ardent/config.yaml");
    let text = fs::read_to_string(&config)?.replace("views: {}", "  html: {}\nviews: {}");
    fs::write(&config, text)?;
    project.write("index.html", "<p>Hello</p>\n")?;
    let run = project.sync()?;
    run.assert_success();
    assert!(
        run.stderr()
            .contains("warning: the format `html` has no plugin"),
        "{}",
        run.stderr()
    );
    assert!(!run.stderr().contains("`rust`"), "{}", run.stderr());
    assert!(!project.structure_path("index.html").exists());
    Ok(())
}

#[test]
fn rust_file_gets_a_structure_file() -> TestResult {
    let project = Project::new()?;
    project.write("src/main.rs", "/// Starts it.\nfn main() {}\n")?;
    let run = project.sync()?;
    run.assert_success();
    let structure = project.structure("src/main.rs")?;
    assert!(structure.contains("format: rust\n"), "{structure}");
    assert!(
        structure.contains(
            "path: src/main.rs#fn main\n    label: fn main\n    kind: fn\n    range: 0..27\n"
        ),
        "{structure}"
    );
    Ok(())
}

#[test]
fn syntax_error_in_one_file_does_not_stop_the_others() -> TestResult {
    let project = Project::new()?;
    project.write("src/lib.rs", "fn run() {}\nfn helper(\n")?;
    project.write("src/main.rs", "fn main() {}\n")?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1), "stderr: {}", run.stderr());
    assert!(
        run.stderr()
            .contains("ardent: src/lib.rs: syntax error at line 2, column "),
        "{}",
        run.stderr()
    );
    assert!(project.structure_path("src/main.rs").is_file());
    assert!(!project.structure_path("src/lib.rs").exists());
    Ok(())
}

#[test]
fn syntax_error_keeps_the_structure_of_the_last_scan() -> TestResult {
    let project = Project::new()?;
    project.write("src/lib.rs", "fn run() {}\n")?;
    project.sync()?.assert_success();
    let before = project.structure("src/lib.rs")?;
    project.write("src/lib.rs", "fn run() {}\nfn helper(\n")?;
    let run = project.sync()?;
    assert_eq!(run.code(), Some(1), "stderr: {}", run.stderr());
    assert!(
        run.stderr().contains("src/lib.rs: syntax error"),
        "{}",
        run.stderr()
    );
    assert_eq!(project.structure("src/lib.rs")?, before);
    project.write("src/lib.rs", "fn run() {}\nfn helper() {}\n")?;
    project.sync()?.assert_success();
    let after = project.structure("src/lib.rs")?;
    assert!(after.contains("path: src/lib.rs#fn helper\n"), "{after}");
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
fn case_only_rename_updates_node_paths_when_another_plugin_is_broken() -> TestResult {
    let project = Project::new()?;
    let before = "SRC/OLD#.rs";
    let after = "Src/Old#.rs";
    let valid = "fn example() {}";
    project.write(before, valid)?;
    project.write("SRC/nested/other.rs", "fn other() {}")?;
    project.write("SRC/notes.md", "# Notes\n")?;
    project.sync()?.assert_success();
    let original = project.structure(before)?;
    let sibling = project.structure("SRC/notes.md")?;
    let manifest = fs::read(project.path(".ardent/plugins/markdown/plugin.yaml"))?;

    fs::rename(project.path("SRC"), project.path("Src"))?;
    fs::rename(project.path("Src/OLD#.rs"), project.path(after))?;
    project.write(".ardent/plugins/markdown/plugin.yaml", "schema: 0")?;
    project.write(after, "fn example(")?;
    let failed = project.sync()?;
    assert_eq!(failed.code(), Some(1));
    assert!(
        failed.stderr().contains("syntax error"),
        "{}",
        failed.stderr()
    );
    assert_eq!(project.structure(before)?, original);
    assert!(
        project
            .structure("Src/nested/other.rs")?
            .contains("Src/nested/other.rs#fn other")
    );

    project.write(after, valid)?;
    let recovered = project.sync()?;
    assert_eq!(recovered.code(), Some(1));
    assert!(
        recovered.stderr().contains("markdown"),
        "{}",
        recovered.stderr()
    );
    assert!(
        recovered.stdout().contains("1 scanned"),
        "{}",
        recovered.stdout()
    );
    let last_good = project.structure(after)?;
    assert!(last_good.contains(r"Src/Old\#.rs#fn example"));
    assert_eq!(project.structure("SRC/notes.md")?, sibling);
    let cache = snapshot(&project.path(".ardent/cache/files"))?;
    assert!(cache.contains_key(Path::new("Src/Old#.rs/structure.yaml")));
    assert!(cache.contains_key(Path::new("Src/nested/other.rs/structure.yaml")));

    let unchanged = project.sync()?;
    assert!(
        unchanged.stdout().contains("0 scanned"),
        "{}",
        unchanged.stdout()
    );

    fs::write(
        project.path(".ardent/plugins/markdown/plugin.yaml"),
        manifest,
    )?;
    project.write(after, "fn example(")?;
    let next = project.sync()?;
    assert_eq!(next.code(), Some(1));
    assert!(next.stderr().contains("syntax error"), "{}", next.stderr());
    assert_eq!(project.structure(after)?, last_good);
    Ok(())
}

#[test]
fn failed_scan_after_case_only_rename_preserves_last_good_structure_through_recovery() -> TestResult
{
    let project = Project::new()?;
    let before = "Old/lib.rs";
    let after = "OLD/Lib.rs";
    project.write(before, "fn example() {}")?;
    project.sync()?.assert_success();
    let original = project.structure(before)?;
    let manifest = project.path(".ardent/plugins/markdown/plugin.yaml");
    let good_manifest = fs::read(&manifest)?;

    fs::rename(project.path("Old"), project.path("OLD"))?;
    fs::rename(project.path("OLD/lib.rs"), project.path(after))?;
    let aliases = project.path(before).exists();
    project.write(after, "fn example(")?;
    fs::write(&manifest, "schema: 0")?;
    let partial = project.sync()?;
    assert_eq!(partial.code(), Some(1));
    assert!(partial.stderr().contains("syntax error"));
    assert_eq!(project.structure(before)?, original);

    fs::write(manifest, good_manifest)?;
    let repaired = project.sync()?;
    assert_eq!(repaired.code(), Some(1));
    assert!(repaired.stderr().contains("syntax error"));
    if aliases {
        assert_eq!(project.structure(after)?, original);
        let cache = snapshot(&project.path(".ardent/cache/files"))?;
        assert!(cache.contains_key(Path::new("OLD/Lib.rs/structure.yaml")));
    } else {
        assert!(!project.structure_path(before).exists());
        assert!(!project.structure_path(after).exists());
    }
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
