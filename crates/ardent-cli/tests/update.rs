mod common;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::SystemTime;

use common::{run_init, snapshot, stage_release, temp_dir};

fn run(binary: &Path, project: &Path, command: &str) -> io::Result<Output> {
    Command::new(binary)
        .arg(command)
        .current_dir(project)
        .output()
}

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn modification_times(root: &Path) -> io::Result<BTreeMap<std::path::PathBuf, SystemTime>> {
    snapshot(root)?
        .into_keys()
        .map(|path| Ok((path.clone(), fs::metadata(root.join(path))?.modified()?)))
        .collect()
}

#[test]
fn updates_installed_formats_without_touching_selection_or_configuration() -> io::Result<()> {
    let release = stage_release()?;
    let project = temp_dir()?;
    let root = project.path();
    let binary = release.executable();
    success(run_init(root, &["--plugins", "markdown"])?);
    let ardent = root.join(".ardent");
    let before = snapshot(&ardent)?;
    assert!(success(run(binary, root, "update")?).contains("up to date"));
    assert_eq!(before, snapshot(&ardent)?);

    let nested = root.join("docs");
    fs::create_dir(&nested)?;
    let config = b"# local choices\nschema: 1\nformats: {}\nviews: {}\nexclude: []\n";
    fs::write(ardent.join("config.yaml"), config)?;
    let plugins = ardent.join("plugins");
    let installed = plugins.join("markdown");
    let other = plugins.join("my-markdown");
    fs::create_dir(&other)?;
    fs::write(other.join("custom"), "mine")?;
    let other_tree = snapshot(&other)?;
    fs::write(installed.join("format.wasm"), "local build")?;
    fs::write(installed.join("obsolete"), "local edits")?;
    let bundled = release.path().join("plugins/markdown");
    fs::create_dir_all(bundled.join("docs/empty"))?;
    fs::write(bundled.join("docs/guide"), "release notes")?;

    assert!(success(run(binary, &nested, "update")?).contains("Updated markdown"));
    assert_eq!(snapshot(&installed)?, snapshot(&bundled)?);
    assert_eq!(fs::read(ardent.join("config.yaml"))?, config);
    assert_eq!(snapshot(&other)?, other_tree);
    assert!(!plugins.join("rust").exists());
    let before = modification_times(&ardent)?;
    assert!(success(run(binary, root, "update")?).contains("up to date"));
    assert_eq!(modification_times(&ardent)?, before);
    Ok(())
}

#[test]
fn preserves_a_view_with_the_same_name_as_a_release_format() -> io::Result<()> {
    let project = temp_dir()?;
    success(run_init(project.path(), &["--plugins", ""])?);
    let dir = project.path().join(".ardent/plugins/markdown");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("plugin.yaml"),
        "schema: 1\nname: markdown\nkind: view\nversion: '1'\ncomponent: view.wasm\n",
    )?;
    fs::write(dir.join("view.wasm"), "third-party")?;
    let before = snapshot(project.path())?;
    success(run(
        Path::new(env!("CARGO_BIN_EXE_ardent")),
        project.path(),
        "update",
    )?);
    assert_eq!(snapshot(project.path())?, before);
    Ok(())
}

fn assert_failed_update(binary: &Path, root: &Path) -> io::Result<()> {
    let package = root.join(".ardent/plugins/markdown");
    let before = snapshot(&package)?;
    let output = run(binary, root, "update")?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("markdown"));
    assert_eq!(snapshot(&package)?, before);
    fs::write(root.join("notes.md"), "# Still usable\n")?;
    assert!(success(run(binary, root, "sync")?).contains("1 scanned"));
    Ok(())
}

#[test]
fn invalid_replacement_keeps_the_working_component() -> io::Result<()> {
    let project = temp_dir()?;
    let release = stage_release()?;
    success(run_init(project.path(), &["--plugins", "markdown"])?);
    fs::write(
        release.path().join("plugins/markdown/format.wasm"),
        "invalid component",
    )?;
    assert_failed_update(release.executable(), project.path())
}

#[cfg(unix)]
#[test]
fn failed_directory_publication_keeps_the_installed_package() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let release = stage_release()?;
    let project = temp_dir()?;
    success(run_init(project.path(), &["--plugins", "markdown"])?);
    fs::write(release.path().join("plugins/markdown/notes"), "new")?;
    let parent = project.path().join(".ardent/plugins");
    let original = fs::metadata(&parent)?.permissions();
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555))?;
    let checked = assert_failed_update(release.executable(), project.path());
    fs::set_permissions(parent, original)?;
    checked
}

#[test]
fn sync_and_update_wait_for_the_project_lock() -> io::Result<()> {
    let release = stage_release()?;
    let project = temp_dir()?;
    let root = project.path();
    success(run_init(root, &["--plugins", "markdown"])?);
    fs::write(root.join("notes.md"), "# Hello\n")?;
    success(run(release.executable(), root, "sync")?);
    let lock = fs::File::options()
        .write(true)
        .open(root.join(".ardent/cache/lock"))?;
    lock.lock()?;
    fs::write(release.path().join("plugins/markdown/notes"), "new")?;
    let start = |command| {
        Command::new(release.executable())
            .arg(command)
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    };
    let mut update = start("update")?;
    let mut sync = start("sync")?;
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(update.try_wait()?.is_none());
    assert!(sync.try_wait()?.is_none());
    assert!(!root.join(".ardent/plugins/markdown/notes").exists());
    lock.unlock()?;
    success(update.wait_with_output()?);
    success(sync.wait_with_output()?);
    assert_eq!(
        snapshot(&release.path().join("plugins/markdown"))?,
        snapshot(&root.join(".ardent/plugins/markdown"))?
    );
    Ok(())
}

#[test]
fn updating_inside_the_release_package_excludes_the_project_installation() -> io::Result<()> {
    let release = stage_release()?;
    let bundled = release.path().join("plugins/markdown");
    let project = bundled.join("example");
    fs::create_dir(&project)?;
    success(
        Command::new(release.executable())
            .args(["init", "--plugins", "markdown"])
            .current_dir(&project)
            .output()?,
    );
    fs::write(bundled.join("notes"), "new")?;
    assert!(success(run(release.executable(), &project, "update")?).contains("Updated markdown"));
    let installed = project.join(".ardent/plugins/markdown");
    assert!(!installed.join("example/.ardent").exists());
    assert_eq!(fs::read(installed.join("notes"))?, b"new");
    assert!(success(run(release.executable(), &project, "update")?).contains("up to date"));
    Ok(())
}
