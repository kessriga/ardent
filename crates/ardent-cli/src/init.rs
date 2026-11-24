//! `ardent init`: make a directory an Ardent project with the initial content of the spec `ardent-layout`.

mod plugins;
mod selection;

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ardent_core::layout::{self, ARDENT_DIR, CACHE_DIR};

use self::plugins::ReleasePlugin;
use crate::project_root;

const GITIGNORE: &str = "/cache/\n";

/// Why `ardent init` created no project.
pub enum InitError {
    Selection(String),
    /// The directory already contains a file or directory named `.ardent`. Nothing changed.
    AlreadyExists(PathBuf),
    /// A parent directory is already a project root. Nothing changed.
    InsideProject(PathBuf),
    /// The file system failed at `path`. Nothing changed: either init had not started, or it removed the `.ardent/` it
    /// had started.
    Io {
        path: PathBuf,
        source: io::Error,
    },
    /// The file system failed at `path`, and init could not remove the `.ardent/` it had started.
    Incomplete {
        path: PathBuf,
        source: io::Error,
        ardent_dir: PathBuf,
        cleanup: io::Error,
    },
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Selection(message) => f.write_str(message),
            Self::AlreadyExists(path) => {
                write!(f, "{} already exists; nothing changed", path.display())
            }
            Self::InsideProject(root) => write!(
                f,
                "this directory is inside the Ardent project at {}; nothing changed",
                root.display()
            ),
            Self::Io { path, source } => write!(f, "{}: {source}; nothing changed", path.display()),
            Self::Incomplete {
                path,
                source,
                ardent_dir,
                cleanup,
            } => write!(
                f,
                "{}: {source}; cannot remove {}: {cleanup}; remove it before you run init again",
                path.display(),
                ardent_dir.display()
            ),
        }
    }
}

/// Creates `.ardent/` with its initial files in `project_dir`, and returns the path of `.ardent/`.
pub fn run(project_dir: &Path, names: Option<&str>) -> Result<PathBuf, InitError> {
    let ardent_dir = project_dir.join(ARDENT_DIR);
    refuse_existing_project(project_dir, &ardent_dir)?;
    let selected = selection::select(names).map_err(InitError::Selection)?;
    if names.is_none() {
        // Another init may have created an enclosing project while the picker was open.
        refuse_existing_project(project_dir, &ardent_dir)?;
    }
    create_ardent_dir(&ardent_dir)?;
    write_initial_files(&ardent_dir, &selected)
        .map_err(|(path, source)| roll_back(&ardent_dir, path, source))?;
    Ok(ardent_dir)
}

/// Checks `project_dir` itself before its parents, so that the error names the nearest project.
fn refuse_existing_project(project_dir: &Path, ardent_dir: &Path) -> Result<(), InitError> {
    if fs::symlink_metadata(ardent_dir).is_ok() {
        return Err(InitError::AlreadyExists(ardent_dir.to_path_buf()));
    }
    let Some(parent) = project_dir.parent() else {
        return Ok(());
    };
    match project_root::find(parent) {
        Ok(Some(root)) => Err(InitError::InsideProject(root)),
        Ok(None) => Ok(()),
        Err((path, source)) => Err(InitError::Io { path, source }),
    }
}

/// `create_dir` fails when `.ardent` exists, so a concurrent init in the same directory cannot also succeed.
fn create_ardent_dir(ardent_dir: &Path) -> Result<(), InitError> {
    fs::create_dir(ardent_dir).map_err(|source| match source.kind() {
        io::ErrorKind::AlreadyExists => InitError::AlreadyExists(ardent_dir.to_path_buf()),
        _ => InitError::Io {
            path: ardent_dir.to_path_buf(),
            source,
        },
    })
}

fn write_initial_files(
    ardent_dir: &Path,
    selected: &[&ReleasePlugin],
) -> Result<(), (PathBuf, io::Error)> {
    let mut config = String::from("schema: 1\nformats:");
    if selected.is_empty() {
        config.push_str(" {}\n");
    } else {
        let source = plugins::release_dir()?;
        config.push('\n');
        for plugin in selected {
            plugin.install(&source, ardent_dir)?;
            config.push_str(&format!("  {}: {{}}\n", plugin.name));
        }
    }
    config.push_str("views: {}\nexclude: []\n");
    for (name, contents) in [(".gitignore", GITIGNORE), ("config.yaml", config.as_str())] {
        layout::write_atomic(ardent_dir, name, contents.as_bytes())
            .map_err(|source| (ardent_dir.join(name), source))?;
    }
    layout::remove_empty_cache(ardent_dir).map_err(|source| (ardent_dir.join(CACHE_DIR), source))
}

/// Removes the `.ardent/` this run created after a failure at `path`.
fn roll_back(ardent_dir: &Path, path: PathBuf, source: io::Error) -> InitError {
    match fs::remove_dir_all(ardent_dir) {
        Ok(()) => InitError::Io { path, source },
        Err(cleanup) => InitError::Incomplete {
            path,
            source,
            ardent_dir: ardent_dir.to_path_buf(),
            cleanup,
        },
    }
}
