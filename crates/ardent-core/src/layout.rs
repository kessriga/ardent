//! Paths inside a project's `.ardent/` directory and the atomic write into it, as the spec `ardent-layout` defines them.

use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use tempfile::NamedTempFile;

/// Name of the directory that marks a project root and holds all Ardent data.
pub const ARDENT_DIR: &str = ".ardent";
/// Derived data, relative to `.ardent/`. Git ignores it, and Ardent rebuilds it when it is missing.
pub const CACHE_DIR: &str = "cache";
/// Temporary files of atomic writes, relative to `cache/`.
const TMP_DIR: &str = "tmp";

/// Writes `contents` to `relative_path` inside `ardent_dir` through a temporary file in `cache/tmp/`, so that the
/// target file has either its old or its new content, even after a crash or a power loss. Creates `cache/tmp/` and the
/// parent directories of the target when they are missing. The file gets the permissions that `fs::write` gives a new
/// file.
///
/// `relative_path` often derives from a source path in a cloned repository, so it must consist of normal names only:
/// an absolute path or a `..` fails with `InvalidInput`. A missing `ardent_dir` fails with `NotFound`, and the call
/// then creates nothing.
pub fn write_atomic(ardent_dir: &Path, relative_path: &str, contents: &[u8]) -> io::Result<()> {
    let target = ardent_dir.join(checked_relative_path(relative_path)?);
    if !fs::metadata(ardent_dir)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("{} is not a directory", ardent_dir.display()),
        ));
    }
    let tmp_dir = tmp_dir(ardent_dir);
    fs::create_dir_all(&tmp_dir)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = create_temp_file(&tmp_dir)?;
    file.write_all(contents)?;
    file.as_file().sync_all()?;
    file.persist(&target).map_err(|err| err.error)?;
    if let Some(parent) = target.parent() {
        sync_dir(parent)?;
    }
    Ok(())
}

/// Removes the empty `cache/tmp/` and `cache/` that `write_atomic` leaves behind, for a `.ardent/` that must not hold a
/// cache yet. Fails when either directory is not empty.
pub fn remove_empty_cache(ardent_dir: &Path) -> io::Result<()> {
    fs::remove_dir(tmp_dir(ardent_dir))?;
    fs::remove_dir(ardent_dir.join(CACHE_DIR))?;
    sync_dir(ardent_dir)
}

fn tmp_dir(ardent_dir: &Path) -> PathBuf {
    ardent_dir.join(CACHE_DIR).join(TMP_DIR)
}

fn checked_relative_path(relative_path: &str) -> io::Result<&Path> {
    let path = Path::new(relative_path);
    let mut components = path.components().peekable();
    if components.peek().is_some() && components.all(|c| matches!(c, Component::Normal(_))) {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{relative_path:?} is not a relative path inside .ardent/"),
        ))
    }
}

/// `tempfile` creates files with mode 0o600; 0o666 minus the umask is what `fs::write` gives a new file.
#[cfg(unix)]
fn create_temp_file(tmp_dir: &Path) -> io::Result<NamedTempFile> {
    use std::os::unix::fs::PermissionsExt;

    tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o666))
        .tempfile_in(tmp_dir)
}

#[cfg(not(unix))]
fn create_temp_file(tmp_dir: &Path) -> io::Result<NamedTempFile> {
    NamedTempFile::new_in(tmp_dir)
}

/// Makes a rename or removal in `dir` survive a power loss.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    fs::File::open(dir)?.sync_all()
}

/// Windows cannot open a directory with `File::open`, and std has no other way to flush one.
#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir_is_empty(ardent_dir: &Path) -> io::Result<bool> {
        Ok(fs::read_dir(tmp_dir(ardent_dir))?.next().is_none())
    }

    #[test]
    fn writes_a_new_file() -> io::Result<()> {
        let ardent_dir = tempfile::tempdir()?;
        write_atomic(ardent_dir.path(), "config.yaml", b"schema: 1\n")?;
        assert_eq!(
            fs::read(ardent_dir.path().join("config.yaml"))?,
            b"schema: 1\n"
        );
        assert!(tmp_dir_is_empty(ardent_dir.path())?);
        Ok(())
    }

    #[test]
    fn replaces_an_existing_file() -> io::Result<()> {
        let ardent_dir = tempfile::tempdir()?;
        fs::write(ardent_dir.path().join("config.yaml"), b"old")?;
        write_atomic(ardent_dir.path(), "config.yaml", b"new")?;
        assert_eq!(fs::read(ardent_dir.path().join("config.yaml"))?, b"new");
        assert!(tmp_dir_is_empty(ardent_dir.path())?);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn gives_a_new_file_the_permissions_of_fs_write() -> io::Result<()> {
        let ardent_dir = tempfile::tempdir()?;
        fs::write(ardent_dir.path().join("plain"), b"")?;
        write_atomic(ardent_dir.path(), "atomic", b"")?;
        let mode = |name: &str| fs::metadata(ardent_dir.path().join(name)).map(|m| m.permissions());
        assert_eq!(mode("atomic")?, mode("plain")?);
        Ok(())
    }

    #[test]
    fn creates_the_parent_directories_of_the_target() -> io::Result<()> {
        let ardent_dir = tempfile::tempdir()?;
        let target = "cache/files/src/engine.rs/structure.yaml";
        write_atomic(ardent_dir.path(), target, b"schema: 1\n")?;
        assert_eq!(fs::read(ardent_dir.path().join(target))?, b"schema: 1\n");
        Ok(())
    }

    #[test]
    fn rejects_paths_that_leave_the_ardent_dir() -> io::Result<()> {
        let outer = tempfile::tempdir()?;
        let ardent_dir = outer.path().join(".ardent");
        fs::create_dir(&ardent_dir)?;
        let absolute = outer.path().join("absolute");
        for path in [
            "../escape",
            "views/../../escape",
            "",
            absolute.to_str().unwrap_or_default(),
        ] {
            let err = write_atomic(&ardent_dir, path, b"").err();
            assert_eq!(
                err.map(|e| e.kind()),
                Some(io::ErrorKind::InvalidInput),
                "{path:?}"
            );
        }
        assert!(!outer.path().join("escape").exists());
        assert!(!absolute.exists());
        Ok(())
    }

    #[test]
    fn creates_nothing_when_the_ardent_dir_is_missing() -> io::Result<()> {
        let outer = tempfile::tempdir()?;
        let missing = outer.path().join(".ardent");
        let err = write_atomic(&missing, "config.yaml", b"").err();
        assert_eq!(err.map(|e| e.kind()), Some(io::ErrorKind::NotFound));
        assert!(!missing.exists());
        Ok(())
    }

    #[test]
    fn removes_the_empty_cache() -> io::Result<()> {
        let ardent_dir = tempfile::tempdir()?;
        write_atomic(ardent_dir.path(), "config.yaml", b"schema: 1\n")?;
        remove_empty_cache(ardent_dir.path())?;
        let names: Vec<_> = fs::read_dir(ardent_dir.path())?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<io::Result<_>>()?;
        assert_eq!(names, ["config.yaml"]);
        Ok(())
    }
}
