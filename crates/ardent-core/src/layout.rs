//! Paths inside a project's `.ardent/` directory and the atomic write into it, as the spec `ardent-layout` defines them.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

/// Name of the directory that marks a project root and holds all Ardent data.
pub const ARDENT_DIR: &str = ".ardent";
/// Derived data, relative to `.ardent/`. Git ignores it, and Ardent rebuilds it when it is missing.
pub const CACHE_DIR: &str = "cache";
/// Temporary files of atomic writes, relative to `cache/`.
const TMP_DIR: &str = "tmp";

/// Writes `contents` to `relative_path` inside `ardent_dir` through a temporary file in `cache/tmp/`, so that the
/// target file has either its old or its new content, even after a crash or a power loss. Creates `cache/tmp/` when it
/// is missing. The file gets the permissions that `fs::write` gives a new file.
pub fn write_atomic(ardent_dir: &Path, relative_path: &str, contents: &[u8]) -> io::Result<()> {
    let tmp_dir = tmp_dir(ardent_dir);
    fs::create_dir_all(&tmp_dir)?;
    let mut file = create_temp_file(&tmp_dir)?;
    file.write_all(contents)?;
    file.as_file().sync_all()?;
    let target = ardent_dir.join(relative_path);
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
