//! Paths inside a project's `.ardent/` directory and the atomic write into it, as the spec `ardent-layout` defines them.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use tempfile::NamedTempFile;

/// Name of the directory that marks a project root and holds all Ardent data.
pub const ARDENT_DIR: &str = ".ardent";
/// Derived data, relative to `.ardent/`. Git ignores it, and Ardent rebuilds it when it is missing.
pub const CACHE_DIR: &str = "cache";
/// Temporary files of atomic writes, relative to `.ardent/`.
pub const TMP_DIR: &str = "cache/tmp";

/// Writes `contents` to `relative_path` inside `ardent_dir` through a temporary file in `cache/tmp/`, so that the
/// target file has either its old or its new content, even after a crash. Creates `cache/tmp/` when it is missing.
pub fn write_atomic(ardent_dir: &Path, relative_path: &str, contents: &[u8]) -> io::Result<()> {
    let tmp_dir = ardent_dir.join(TMP_DIR);
    fs::create_dir_all(&tmp_dir)?;
    let mut file = NamedTempFile::new_in(&tmp_dir)?;
    file.write_all(contents)?;
    file.as_file().sync_all()?;
    file.persist(ardent_dir.join(relative_path))
        .map_err(|err| err.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir_is_empty(ardent_dir: &Path) -> io::Result<bool> {
        Ok(fs::read_dir(ardent_dir.join(TMP_DIR))?.next().is_none())
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
}
