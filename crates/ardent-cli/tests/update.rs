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

#[test]
fn invalid_config_prevents_replacement_and_recovery() -> io::Result<()> {
    let project = temp_dir()?;
    let root = project.path();
    let binary = Path::new(env!("CARGO_BIN_EXE_ardent"));
    success(run_init(root, &["--plugins", "markdown,rust"])?);
    let ardent = root.join(".ardent");
    fs::write(ardent.join("plugins/markdown/local"), "keep")?;
    let marker = ardent.join("plugins/.ardent-update-rust/ardent-plugin-update-v1");
    fs::create_dir_all(&marker)?;
    fs::rename(ardent.join("plugins/rust"), marker.join("previous"))?;
    for config in [
        "schema: 2\n",
        "schema: 1\nformats: {}\nviews: {}\nexclude: ['[z-a]']\n",
    ] {
        fs::write(ardent.join("config.yaml"), config)?;
        let before = snapshot(&ardent)?;
        let output = run(binary, root, "update")?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("config.yaml"));
        assert_eq!(before, snapshot(&ardent)?);
    }
    Ok(())
}

#[test]
fn check_reports_a_reserved_third_party_plugin() -> io::Result<()> {
    let project = temp_dir()?;
    let root = project.path();
    success(run_init(root, &["--plugins", "markdown"])?);
    let dir = root.join(".ardent/plugins/structure");
    fs::create_dir(&dir)?;
    fs::write(dir.join("plugin.yaml"), "schema: 1\n")?;
    fs::write(root.join(".ardent/plugins/markdown/local"), "keep")?;
    let before = snapshot(root)?;
    let output = run(Path::new(env!("CARGO_BIN_EXE_ardent")), root, "update")?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("structure"));
    assert_eq!(snapshot(root)?, before);
    Ok(())
}

#[test]
fn recovery_restores_into_an_empty_path_and_keeps_unrelated_files() -> io::Result<()> {
    let project = temp_dir()?;
    let root = project.path();
    success(run_init(root, &["--plugins", "markdown"])?);
    let installed = root.join(".ardent/plugins/markdown");
    fs::write(installed.join("local"), "previous")?;
    let journal = installed.with_file_name(".ardent-update-markdown");
    let marker = journal.join("ardent-plugin-update-v1");
    fs::create_dir_all(&marker)?;
    fs::rename(&installed, marker.join("previous"))?;
    fs::create_dir(&installed)?;
    fs::write(journal.join(".DS_Store"), "keep")?;
    fs::write(marker.join("note"), "also keep")?;
    fs::write(root.join("notes.md"), "# Hello\n")?;
    let binary = Path::new(env!("CARGO_BIN_EXE_ardent"));
    for _ in 0..2 {
        success(run(binary, root, "sync")?);
        assert_eq!(fs::read(installed.join("local"))?, b"previous");
        assert_eq!(fs::read(journal.join(".DS_Store"))?, b"keep");
        assert_eq!(fs::read(marker.join("note"))?, b"also keep");
    }
    Ok(())
}

#[test]
fn update_ignores_a_manifest_only_in_a_recognized_recovery_directory() -> io::Result<()> {
    let project = temp_dir()?;
    let root = project.path();
    let binary = Path::new(env!("CARGO_BIN_EXE_ardent"));
    success(run_init(root, &["--plugins", "markdown"])?);
    let installed = root.join(".ardent/plugins/markdown");
    let journal = installed.with_file_name(".ardent-update-markdown");
    let marker = journal.join("ardent-plugin-update-v1");
    fs::create_dir(&journal)?;
    let unrelated = journal.join("plugin.yaml");
    fs::write(&unrelated, "reader's copy")?;

    let output = run(binary, root, "update")?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(".ardent-update-markdown"));

    fs::create_dir(&marker)?;
    fs::rename(&installed, marker.join("previous"))?;
    for command in ["update", "sync", "update"] {
        success(run(binary, root, command)?);
        assert!(installed.join("format.wasm").is_file());
        assert_eq!(fs::read_to_string(&unrelated)?, "reader's copy");
    }
    Ok(())
}

#[test]
fn ambiguous_recovery_keeps_both_copies_and_scans_healthy_formats() -> io::Result<()> {
    let project = temp_dir()?;
    let root = project.path();
    let binary = Path::new(env!("CARGO_BIN_EXE_ardent"));
    success(run_init(root, &["--plugins", "markdown,rust"])?);
    fs::write(root.join("notes.md"), "# Old\n")?;
    success(run(binary, root, "sync")?);
    let ardent = root.join(".ardent");
    let installed = ardent.join("plugins/markdown");
    let marker = installed
        .with_file_name(".ardent-update-markdown")
        .join("ardent-plugin-update-v1");
    fs::create_dir_all(&marker)?;
    fs::rename(&installed, marker.join("previous"))?;
    fs::create_dir(&installed)?;
    fs::write(installed.join("local"), "keep this too")?;
    fs::write(root.join("code.rs"), "fn okay() {}\n")?;
    let output = run(binary, root, "sync")?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("previous"));
    assert!(marker.join("previous/plugin.yaml").is_file());
    assert!(installed.join("local").is_file());
    assert!(ardent.join("cache/files/code.rs/structure.yaml").is_file());
    assert!(ardent.join("cache/files/notes.md/structure.yaml").is_file());
    Ok(())
}

#[cfg(unix)]
#[test]
fn update_preserves_shared_access_under_a_private_umask() -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let release = stage_release()?;
    let project = temp_dir()?;
    let root = project.path();
    let binary = release.executable();
    success(run_init(root, &["--plugins", "markdown"])?);
    let installed = root.join(".ardent/plugins/markdown");
    let ardent = root.join(".ardent");
    fs::set_permissions(&ardent, fs::Permissions::from_mode(0o775))?;
    assert!(!ardent.join("cache").exists());
    fs::set_permissions(&installed, fs::Permissions::from_mode(0o775))?;
    fs::set_permissions(
        installed.join("plugin.yaml"),
        fs::Permissions::from_mode(0o640),
    )?;
    fs::create_dir_all(release.path().join("plugins/markdown/docs"))?;
    fs::write(release.path().join("plugins/markdown/docs/guide"), "new")?;
    success(
        Command::new("sh")
            .args(["-c", "umask 077 && exec \"$1\" update", "sh"])
            .arg(binary)
            .current_dir(root)
            .output()?,
    );
    for (path, mode) in [
        ("", 0o775),
        ("plugin.yaml", 0o640),
        ("docs", 0o775),
        ("docs/guide", 0o664),
    ] {
        assert_eq!(
            fs::metadata(installed.join(path))?.permissions().mode() & 0o777,
            mode,
            "{path}"
        );
    }
    for (path, mode) in [
        ("cache", 0o775),
        ("cache/tmp", 0o775),
        ("cache/lock", 0o664),
    ] {
        assert_eq!(
            fs::metadata(ardent.join(path))?.permissions().mode() & 0o777,
            mode,
            "{path}"
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn init_and_update_reject_linked_package_and_release_roots() -> io::Result<()> {
    use std::os::unix::fs::symlink;
    for name in ["plugins/markdown", "plugins"] {
        let release = stage_release()?;
        let project = temp_dir()?;
        let fresh = temp_dir()?;
        success(run_init(project.path(), &["--plugins", "markdown"])?);
        let root = release.path().join(name);
        let moved = release.path().join("bundle");
        fs::rename(&root, &moved)?;
        symlink(&moved, &root)?;
        for dangling in [false, true] {
            if dangling {
                fs::rename(&moved, moved.with_extension("away"))?;
            }
            let init = Command::new(release.executable())
                .args(["init", "--plugins", "markdown"])
                .current_dir(fresh.path())
                .output()?;
            assert!(!init.status.success());
            assert!(!fresh.path().join(".ardent").try_exists()?);
            assert!(
                !run(release.executable(), project.path(), "update")?
                    .status
                    .success()
            );
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn update_keeps_named_access_and_inherits_it_for_new_paths() -> io::Result<()> {
    use exacl::{AclEntry, Flag, Perm};

    let project = temp_dir()?;
    let release = stage_release()?;
    let root = project.path();
    success(run_init(root, &["--plugins", "markdown"])?);
    let installed = root.join(".ardent/plugins/markdown");
    let inheritable = AclEntry::allow_group(
        "everyone",
        Perm::READ,
        Flag::DIRECTORY_INHERIT | Flag::FILE_INHERIT,
    );
    exacl::setfacl(&[&installed], &[inheritable], None)?;
    let file = installed.join("plugin.yaml");
    exacl::setfacl(
        &[&file],
        &[AclEntry::allow_group("everyone", Perm::READ, None)],
        None,
    )?;
    let reference = installed.join("reference");
    fs::create_dir(&reference)?;
    fs::write(reference.join("guide"), "reference")?;
    let expected_directory = exacl::getfacl(&reference, None)?;
    let expected_file = exacl::getfacl(reference.join("guide"), None)?;
    let existing_file = exacl::getfacl(&file, None)?;
    assert!(!expected_file.is_empty());
    let docs = release.path().join("plugins/markdown/docs");
    fs::create_dir(&docs)?;
    fs::write(docs.join("guide"), "new")?;
    success(run(release.executable(), root, "update")?);
    assert_eq!(exacl::getfacl(&file, None)?, existing_file);
    assert_eq!(
        exacl::getfacl(installed.join("docs"), None)?,
        expected_directory
    );
    assert_eq!(
        exacl::getfacl(installed.join("docs/guide"), None)?,
        expected_file
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn update_preserves_posix_acls_and_the_inherited_mask() -> io::Result<()> {
    check_posix_acl_inheritance(0o775, 4)?;
    check_posix_acl_inheritance(0o755, 7)
}

#[cfg(target_os = "linux")]
fn check_posix_acl_inheritance(parent_mode: u32, mask: u16) -> io::Result<()> {
    use rustix::fs::{self as xattr, XattrFlags};
    use std::os::unix::fs::PermissionsExt;

    // Linux POSIX ACL version 2: owner, named user, owning group, effective mask, others.
    let mut acl = 2_u32.to_le_bytes().to_vec();
    for (tag, permissions, id) in [
        (1_u16, 7_u16, u32::MAX),
        (2, 6, 65534),
        (4, 7, u32::MAX),
        (16, mask, u32::MAX),
        (32, 0, u32::MAX),
    ] {
        acl.extend(tag.to_le_bytes());
        acl.extend(permissions.to_le_bytes());
        acl.extend(id.to_le_bytes());
    }
    let project = temp_dir()?;
    let release = stage_release()?;
    let root = project.path();
    success(run_init(root, &["--plugins", "markdown"])?);
    let ardent = root.join(".ardent");
    let installed = ardent.join("plugins/markdown");
    let manifest = installed.join("plugin.yaml");
    for parent in [&installed, &ardent] {
        fs::set_permissions(parent, fs::Permissions::from_mode(parent_mode))?;
        xattr::setxattr(
            parent,
            "system.posix_acl_default",
            &acl,
            XattrFlags::empty(),
        )?;
    }
    xattr::setxattr(
        &manifest,
        "system.posix_acl_access",
        &acl,
        XattrFlags::empty(),
    )?;
    let reference = installed.join("reference");
    fs::create_dir(&reference)?;
    fs::write(reference.join("guide"), "reference")?;
    let read_acl = |path: &Path, key: &str| -> io::Result<Vec<u8>> {
        let mut value = vec![0; xattr::getxattr_query(path, key)?];
        xattr::getxattr(path, key, value.as_mut_slice())?;
        Ok(value)
    };
    let key = "system.posix_acl_access";
    let new_directory_acl = read_acl(&reference, key)?;
    let new_file_acl = read_acl(&reference.join("guide"), key)?;
    let docs = release.path().join("plugins/markdown/docs");
    fs::create_dir(&docs)?;
    fs::write(docs.join("guide"), "guide")?;
    success(run(release.executable(), root, "update")?);
    assert_eq!(read_acl(&manifest, key)?, acl);
    assert_eq!(read_acl(&ardent.join("cache"), key)?, new_directory_acl);
    assert_eq!(read_acl(&ardent.join("cache/lock"), key)?, new_file_acl);
    assert_eq!(read_acl(&installed.join("docs"), key)?, new_directory_acl);
    assert_eq!(
        read_acl(&installed.join("docs"), "system.posix_acl_default")?,
        acl
    );
    assert_eq!(read_acl(&installed.join("docs/guide"), key)?, new_file_acl);
    Ok(())
}
