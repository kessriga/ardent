//! Runs `ardent init` in temporary directories and compares the directory trees before and after.

mod common;
#[path = "init/package.rs"]
mod package;
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "common/terminal.rs"]
mod pty;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ardent_core::config::Config;
use common::{run_init, snapshot, stage_release, temp_dir};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "init/terminal.rs"]
mod terminal;

/// Runs `ardent init` in `dir` and checks that it fails with `message` and changes nothing under `root`.
fn assert_refused(dir: &Path, root: &Path, message: &str) -> io::Result<()> {
    let before = snapshot(root)?;
    let output = run_init(dir, &[])?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains(message), "stderr: {stderr}");
    assert_eq!(snapshot(root)?, before);
    Ok(())
}

#[test]
fn empty_selection_creates_only_the_initial_files() -> io::Result<()> {
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
        Some(b"schema: 1\nformats: {}\nviews: {}\nexclude: []\n".to_vec()),
    );

    let output = run_init(project.path(), &["--plugins", ""])?;

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
fn rust_selection_installs_the_release_and_scans_only_rust()
-> Result<(), Box<dyn std::error::Error>> {
    let project = temp_dir()?;
    let dir = project.path();
    fs::write(dir.join("main.rs"), "fn main() {}\n")?;
    fs::write(dir.join("README.md"), "# Readme\n")?;
    let output = run_init(dir, &["--plugins", "rust"])?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let installed = dir.join(".ardent/plugins");
    let release = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/rust");
    assert_eq!(fs::read_dir(&installed)?.count(), 1);
    assert_eq!(snapshot(&installed.join("rust"))?, snapshot(&release)?);
    let config = Config::from_yaml(&fs::read_to_string(dir.join(".ardent/config.yaml"))?)?;
    assert_eq!(config.formats.keys().collect::<Vec<_>>(), ["rust"]);
    assert_eq!(config.formats["rust"].files, None);

    let sync = Command::new(env!("CARGO_BIN_EXE_ardent"))
        .arg("sync")
        .current_dir(dir)
        .output()?;
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );
    assert!(
        dir.join(".ardent/cache/files/main.rs/structure.yaml")
            .is_file()
    );
    assert!(!dir.join(".ardent/cache/files/README.md").exists());
    Ok(())
}

#[test]
fn requires_a_selection_outside_a_terminal() -> io::Result<()> {
    let project = temp_dir()?;
    assert_refused(project.path(), project.path(), "--plugins")
}

#[test]
fn rejects_unknown_names_before_installing_any_package() -> io::Result<()> {
    let project = temp_dir()?;
    let output = run_init(project.path(), &["--plugins", "markdown,html"])?;
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    for name in ["html", "markdown", "rust"] {
        assert!(error.contains(name), "{error}");
    }
    assert!(!project.path().join(".ardent").exists());
    Ok(())
}

#[test]
fn incomplete_release_rolls_back_and_allows_retry() -> io::Result<()> {
    let release = stage_release()?;
    let project = temp_dir()?;
    let markdown = release.path().join("plugins/markdown");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/markdown");
    fs::remove_file(markdown.join("format.wasm"))?;
    let mut init = Command::new(release.executable());
    init.args(["init", "--plugins", "markdown"])
        .current_dir(project.path());
    let failure = init.output()?;
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("format.wasm"));
    assert!(!project.path().join(".ardent").exists());
    fs::copy(source.join("format.wasm"), markdown.join("format.wasm"))?;
    let retry = init.output()?;
    assert!(
        retry.status.success(),
        "{}",
        String::from_utf8_lossy(&retry.stderr)
    );
    assert_eq!(
        snapshot(&markdown)?,
        snapshot(&project.path().join(".ardent/plugins/markdown"))?
    );
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
    let release = stage_release()?;
    let mut child = Command::new(release.executable())
        .args(["init", "--plugins", "markdown,rust"])
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
