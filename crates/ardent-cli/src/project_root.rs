//! The project root: the nearest directory, from a start directory upward, that contains a `.ardent/` directory.

use std::path::{Path, PathBuf};

use ardent_core::layout::ARDENT_DIR;

/// Returns the nearest of `start` and its ancestors that contains a `.ardent/` directory. A file named `.ardent` does
/// not make a project root.
pub fn find(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(ARDENT_DIR).is_dir())
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;

    use super::*;

    #[test]
    fn finds_the_nearest_project() -> io::Result<()> {
        let outer = tempfile::tempdir()?;
        let inner = outer.path().join("inner");
        fs::create_dir(outer.path().join(ARDENT_DIR))?;
        fs::create_dir_all(inner.join(ARDENT_DIR))?;
        fs::create_dir(inner.join("docs"))?;
        assert_eq!(find(&inner.join("docs")), Some(inner));
        Ok(())
    }

    #[test]
    fn skips_a_file_named_ardent() -> io::Result<()> {
        let outer = tempfile::tempdir()?;
        let inner = outer.path().join("inner");
        fs::create_dir(outer.path().join(ARDENT_DIR))?;
        fs::create_dir(&inner)?;
        fs::write(inner.join(ARDENT_DIR), "")?;
        assert_eq!(find(&inner), Some(outer.path().to_path_buf()));
        Ok(())
    }
}
