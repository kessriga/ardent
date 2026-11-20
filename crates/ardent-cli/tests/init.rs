//! Runs `ardent init` in temporary directories and compares the directory trees before and after.

mod common;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use common::{snapshot, temp_dir};

/// The configuration from the scenario "Files after init" in the spec `ardent-layout`.
const INITIAL_CONFIG: &str = r#"schema: 1
formats:
  markdown:
    files:
      - "*.md"
      - "*.markdown"
  rust:
    files:
      - "*.rs"
views: {}
exclude: []
"#;

fn run_init(dir: &Path) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_ardent"))
        .arg("init")
        .current_dir(dir)
        .output()
}

/// Runs `ardent init` in `dir` and checks that it fails with `message` and changes nothing under `root`.
fn assert_refused(dir: &Path, root: &Path, message: &str) -> io::Result<()> {
    let before = snapshot(root)?;
    let output = run_init(dir)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains(message), "stderr: {stderr}");
    assert_eq!(snapshot(root)?, before);
    Ok(())
}

#[test]
fn creates_only_the_initial_files() -> io::Result<()> {
    let project = temp_dir()?;
    fs::write(project.path().join("README.md"), "# Readme\n")?;
    fs::create_dir(project.path().join("src"))?;
    fs::write(project.path().join("src/main.rs"), "fn main() {}\n")?;
    let mut expected = snapshot(project.path())?;
    expected.insert(PathBuf::from(".ardent"), None);
    expected.insert(
        PathBuf::from(".ardent/.gitignore"),
        Some(b"/cache/\n".to_vec()),
    );
    expected.insert(
        PathBuf::from(".ardent/config.yaml"),
        Some(INITIAL_CONFIG.as_bytes().to_vec()),
    );

    let output = run_init(project.path())?;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(".ardent"));
    assert_eq!(snapshot(project.path())?, expected);
    Ok(())
}

#[test]
fn refuses_an_existing_ardent_directory() -> io::Result<()> {
    let project = temp_dir()?;
    fs::create_dir(project.path().join(".ardent"))?;
    fs::write(project.path().join(".ardent/config.yaml"), "schema: 1\n")?;
    assert_refused(project.path(), project.path(), ".ardent already exists")
}

#[test]
fn refuses_a_file_named_ardent() -> io::Result<()> {
    let project = temp_dir()?;
    fs::write(project.path().join(".ardent"), "notes\n")?;
    assert_refused(project.path(), project.path(), ".ardent already exists")
}

#[test]
fn refuses_a_directory_inside_a_project() -> io::Result<()> {
    let project = temp_dir()?;
    fs::create_dir(project.path().join(".ardent"))?;
    fs::create_dir(project.path().join("docs"))?;
    let root = fs::canonicalize(project.path())?;
    let message = format!("inside the Ardent project at {}", root.display());
    assert_refused(&project.path().join("docs"), project.path(), &message)
}

#[test]
fn names_the_project_in_the_directory_before_one_above_it() -> io::Result<()> {
    let project = temp_dir()?;
    fs::create_dir(project.path().join(".ardent"))?;
    fs::create_dir_all(project.path().join("sub/.ardent"))?;
    assert_refused(
        &project.path().join("sub"),
        project.path(),
        "sub/.ardent already exists",
    )
}

#[test]
fn succeeds_when_stdout_is_closed() -> io::Result<()> {
    let project = temp_dir()?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_ardent"))
        .arg("init")
        .current_dir(project.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    drop(child.stdout.take());
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(project.path().join(".ardent/config.yaml").is_file());
    Ok(())
}
