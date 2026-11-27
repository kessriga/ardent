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

#[test]
fn concurrent_sync_and_update_leave_a_complete_package() -> io::Result<()> {
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
    let update = start("update")?;
    let sync = start("sync")?;
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

#[test]
fn reports_each_updated_plugin_even_when_another_fails() -> io::Result<()> {
    let project = temp_dir()?;
    let release = stage_release()?;
    let root = project.path();
    success(run_init(root, &["--plugins", "markdown,rust"])?);
    let installed = root.join(".ardent/plugins");
    let bundled = release.path().join("plugins");
    fs::write(bundled.join("markdown/notes"), "new")?;
    let rust = snapshot(&installed.join("rust"))?;
    fs::write(bundled.join("rust/format.wasm"), "invalid replacement")?;
    let output = run(release.executable(), root, "update")?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Updated markdown"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot update rust"));
    assert_eq!(fs::read(installed.join("markdown/notes"))?, b"new");
    assert_eq!(snapshot(&installed.join("rust"))?, rust);

    fs::copy(
        installed.join("rust/format.wasm"),
        bundled.join("rust/format.wasm"),
    )?;
    fs::write(bundled.join("rust/notes"), "new")?;
    fs::remove_file(bundled.join("markdown/format.wasm"))?;
    let output = run(release.executable(), root, "update")?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Updated rust"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot update markdown"));
    assert_eq!(fs::read(installed.join("rust/notes"))?, b"new");
    Ok(())
}

#[test]
fn current_project_can_be_checked_with_a_read_only_lock() -> io::Result<()> {
    let project = temp_dir()?;
    let root = project.path();
    let exe = Path::new(env!("CARGO_BIN_EXE_ardent"));
    success(run_init(root, &["--plugins", "markdown"])?);
    success(run(exe, root, "sync")?);
    let lock = root.join(".ardent/cache/lock");
    let original = fs::metadata(&lock)?.permissions();
    let mut read_only = original.clone();
    read_only.set_readonly(true);
    fs::set_permissions(&lock, read_only)?;
    let result = run(exe, root, "update");
    fs::set_permissions(&lock, original)?;
    assert!(success(result?).contains("up to date"));
    Ok(())
}

#[test]
fn sync_and_update_recover_after_cache_removal_at_either_rename() -> io::Result<()> {
    let binary = Path::new(env!("CARGO_BIN_EXE_ardent"));
    for command in ["sync", "update"] {
        for published in [false, true] {
            let project = temp_dir()?;
            let root = project.path();
            success(run_init(root, &["--plugins", "markdown"])?);
            fs::write(root.join("notes.md"), "# Notes\n")?;
            success(run(binary, root, "sync")?);
            let ardent = root.join(".ardent");
            let installed = ardent.join("plugins/markdown");
            let journal = ardent.join("plugins/.ardent-update-markdown");
            let marker = journal.join("ardent-plugin-update-v1");
            fs::create_dir_all(&marker)?;
            let previous = marker.join("previous");
            if published {
                fs::create_dir(&previous)?;
                fs::write(previous.join("obsolete"), "old package")?;
            } else {
                fs::rename(&installed, &previous)?;
            }
            fs::remove_dir_all(ardent.join("cache"))?;
            success(run(binary, root, command)?);
            assert!(!journal.try_exists()?);
            assert!(installed.join("format.wasm").is_file());
            assert!(!installed.join("obsolete").exists());
            success(run(binary, root, "sync")?);
            assert!(ardent.join("cache/files/notes.md/structure.yaml").is_file());
        }
    }
    Ok(())
}
