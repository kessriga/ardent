use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

use crate::common::{snapshot, stage_release, temp_dir};

#[test]
fn copies_all_files_in_the_selected_package() -> io::Result<()> {
    let release = stage_release()?;
    let project = temp_dir()?;
    let package = release.path().join("plugins/markdown");
    let data = package.join("data").join("queries");
    fs::create_dir_all(&data)?;
    fs::write(data.join(".headings"), b"\x00\xff")?;
    fs::write(data.join("café"), b"query")?;
    #[cfg(target_os = "linux")]
    {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        fs::write(data.join(OsStr::from_bytes(b"query-\xff")), b"contents")?;
    }
    let output = Command::new(release.executable())
        .args(["init", "--plugins", "markdown"])
        .current_dir(project.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        snapshot(&project.path().join(".ardent/plugins/markdown"))?,
        snapshot(&package)?
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn finds_the_release_when_launched_through_a_symlink() -> io::Result<()> {
    use std::os::unix::fs as unix_fs;

    let release = stage_release()?;
    let project = temp_dir()?;
    let bin = release.path().join("bin");
    fs::create_dir_all(bin.join("plugins"))?;
    let link = bin.join("ardent");
    unix_fs::symlink(release.executable(), &link)?;
    let output = Command::new(link)
        .args(["init", "--plugins", "rust"])
        .current_dir(project.path())
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        project
            .path()
            .join(".ardent/plugins/rust/format.wasm")
            .is_file()
    );
    Ok(())
}

fn assert_invalid_package(executable: &Path, bad_entry: &str) -> io::Result<()> {
    let project = temp_dir()?;
    let output = Command::new(executable)
        .args(["init", "--plugins", "markdown,rust"])
        .current_dir(project.path())
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains(bad_entry), "{stderr}");
    assert!(!project.path().join(".ardent").exists());
    Ok(())
}

#[test]
fn rejects_a_directory_in_place_of_a_required_file() -> io::Result<()> {
    let release = stage_release()?;
    let readme = release.path().join("plugins/rust/README.md");
    fs::remove_file(&readme)?;
    fs::create_dir(&readme)?;
    assert_invalid_package(release.executable(), "README.md")
}

#[cfg(unix)]
#[test]
fn rejects_a_symlink_in_a_package() -> io::Result<()> {
    use std::os::unix::fs as unix_fs;

    let release = stage_release()?;
    let package = release.path().join("plugins/rust");
    let link = package.join("linked-release");
    unix_fs::symlink(release.path(), &link)?;
    assert_invalid_package(release.executable(), "linked-release")
}

#[cfg(target_os = "linux")]
#[test]
fn rejects_a_named_pipe_in_a_package() -> io::Result<()> {
    use rustix::fs::{self as posix, CWD, Mode};

    let release = stage_release()?;
    let pipe = release.path().join("plugins/rust/data.pipe");
    posix::mkfifoat(CWD, &pipe, Mode::RUSR | Mode::WUSR)?;
    assert_invalid_package(release.executable(), "data.pipe")
}

#[test]
fn requires_exact_spelling_for_release_files() -> io::Result<()> {
    let release = stage_release()?;
    let package = release.path().join("plugins/rust");
    for (expected, renamed) in [
        ("plugin.yaml", "Plugin.yaml"),
        ("format.wasm", "Format.wasm"),
        ("README.md", "Readme.md"),
    ] {
        fs::rename(package.join(expected), package.join(renamed))?;
        assert_invalid_package(release.executable(), expected)?;
        fs::rename(package.join(renamed), package.join(expected))?;
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn installs_inside_a_selected_package_without_copying_its_output() -> io::Result<()> {
    use std::os::unix::fs as unix_fs;

    let release = stage_release()?;
    let external = temp_dir()?;
    let package = external.path();
    for name in ["plugin.yaml", "format.wasm", "README.md"] {
        fs::write(package.join(name), name)?;
    }
    let source = release.path().join("plugins/rust");
    fs::rename(&source, source.with_file_name("original-rust"))?;
    unix_fs::symlink(package, source)?;
    let project = package.join("example");
    fs::create_dir(&project)?;
    fs::write(project.join("example.rs"), "fn main() {}")?;
    let other = package.join("fixture/.ardent");
    fs::create_dir_all(&other)?;
    fs::write(other.join("config.yaml"), "schema: 1")?;
    let expected = snapshot(package)?;

    let output = Command::new(release.executable())
        .args(["init", "--plugins", "markdown,rust"])
        .current_dir(&project)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(snapshot(&project.join(".ardent/plugins/rust"))?, expected);
    Ok(())
}
