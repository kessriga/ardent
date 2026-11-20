//! `ardent init`: make a directory an Ardent project with the initial content of the spec `ardent-layout`.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ardent_core::layout::{self, ARDENT_DIR, CACHE_DIR, TMP_DIR};

use crate::project_root;

const GITIGNORE: &str = "/cache/\n";
const CONFIG: &str = r#"schema: 1
formats:
  markdown:
    files:
      - "*.md"
      - "*.markdown"
  rust:
    files:
      - "*.rs"
views: {}
exclude: []
"#;

/// Why `ardent init` created no project. In every case, no file changed.
pub enum InitError {
    /// The directory already contains a file or directory named `.ardent`.
    AlreadyExists(PathBuf),
    /// A parent directory is already a project root.
    InsideProject(PathBuf),
    /// The file system failed while init built `.ardent/`, and init removed it again.
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists(path) => {
                write!(f, "{} already exists; nothing changed", path.display())
            }
            Self::InsideProject(root) => write!(
                f,
                "this directory is inside the Ardent project at {}; nothing changed",
                root.display()
            ),
            Self::Io { path, source } => write!(f, "cannot create {}: {source}", path.display()),
        }
    }
}

/// Creates `.ardent/` with its initial files in `project_dir`, and returns the path of `.ardent/`.
pub fn run(project_dir: &Path) -> Result<PathBuf, InitError> {
    if let Some(root) = project_dir.parent().and_then(project_root::find) {
        return Err(InitError::InsideProject(root));
    }
    let ardent_dir = project_dir.join(ARDENT_DIR);
    fs::create_dir(&ardent_dir).map_err(|source| match source.kind() {
        io::ErrorKind::AlreadyExists => InitError::AlreadyExists(ardent_dir.clone()),
        _ => InitError::Io {
            path: ardent_dir.clone(),
            source,
        },
    })?;
    if let Err(source) = write_initial_files(&ardent_dir) {
        // This run created the directory, so removing it removes only this run's work. A failed removal must not hide
        // the write error.
        let _ = fs::remove_dir_all(&ardent_dir);
        return Err(InitError::Io {
            path: ardent_dir,
            source,
        });
    }
    Ok(ardent_dir)
}

fn write_initial_files(ardent_dir: &Path) -> io::Result<()> {
    layout::write_atomic(ardent_dir, ".gitignore", GITIGNORE.as_bytes())?;
    layout::write_atomic(ardent_dir, "config.yaml", CONFIG.as_bytes())?;
    // After init, `.ardent/` holds only the two files; the first sync creates the cache again.
    fs::remove_dir(ardent_dir.join(TMP_DIR))?;
    fs::remove_dir(ardent_dir.join(CACHE_DIR))
}
