//! The project root: the nearest directory, from a start directory upward, that contains a `.ardent/` directory.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ardent_core::layout::ARDENT_DIR;

/// Returns the nearest of `start` and its ancestors that contains a `.ardent/` directory. A file named `.ardent` does
/// not make a project root. When a directory cannot be checked for a reason other than a missing entry, such as a
/// denied permission, the search fails with the path it could not check, so that a command never picks an outer root
/// by mistake.
pub fn find(start: &Path) -> Result<Option<PathBuf>, (PathBuf, io::Error)> {
    for dir in start.ancestors() {
        let candidate = dir.join(ARDENT_DIR);
        match fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_dir() => return Ok(Some(dir.to_path_buf())),
            Ok(_) => {}
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) => {}
            Err(err) => return Err((candidate, err)),
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_nearest_project() -> io::Result<()> {
        let outer = tempfile::tempdir()?;
        let inner = outer.path().join("inner");
        fs::create_dir(outer.path().join(ARDENT_DIR))?;
        fs::create_dir_all(inner.join(ARDENT_DIR))?;
        fs::create_dir(inner.join("docs"))?;
        assert_eq!(find(&inner.join("docs")).ok(), Some(Some(inner)));
        Ok(())
    }

    #[test]
    fn skips_a_file_named_ardent() -> io::Result<()> {
        let outer = tempfile::tempdir()?;
        let inner = outer.path().join("inner");
        fs::create_dir(outer.path().join(ARDENT_DIR))?;
        fs::create_dir(&inner)?;
        fs::write(inner.join(ARDENT_DIR), "")?;
        assert_eq!(find(&inner).ok(), Some(Some(outer.path().to_path_buf())));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn fails_on_a_directory_that_cannot_be_searched() -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let outer = tempfile::tempdir()?;
        let locked = outer.path().join("locked");
        fs::create_dir_all(locked.join("inner"))?;
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o600))?;
        let result = find(&locked.join("inner"));
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o700))?;
        let (path, err) = result
            .err()
            .ok_or_else(|| io::Error::other("the search succeeded"))?;
        assert_eq!(path, locked.join("inner").join(ARDENT_DIR));
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        Ok(())
    }
}
