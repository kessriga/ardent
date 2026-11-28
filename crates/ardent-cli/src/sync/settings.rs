use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use ardent_core::config::{self, CompilationCache, Config, ConfigError};
use ardent_core::layout::ARDENT_DIR;
use ardent_core::structure;

use crate::xdg;

use super::{SyncError, project_patterns};

pub(super) fn compilation_dir(root: &Path, scope: CompilationCache) -> Option<PathBuf> {
    let compiled = xdg::home("XDG_CACHE_HOME", ".cache")?.join("ardent/compiled");
    let root = root.canonicalize().ok()?;
    let directory = match scope {
        CompilationCache::User => compiled.join("shared"),
        CompilationCache::Project => {
            let id = structure::content_hash(root.as_os_str().as_encoded_bytes());
            compiled.join("projects").join(id)
        }
    };
    let resolved = resolve_cache_directory(&directory)?;
    (!resolved.starts_with(root)).then_some(resolved)
}

// Resolve aliases before allowing Wasmtime to create any missing directories.
fn resolve_cache_directory(directory: &Path) -> Option<PathBuf> {
    for ancestor in directory.ancestors() {
        match ancestor.canonicalize() {
            Ok(path) => {
                let suffix = directory.strip_prefix(ancestor).ok()?;
                if !path.is_dir()
                    || !suffix
                        .components()
                        .all(|part| matches!(part, Component::Normal(_)))
                {
                    return None;
                }
                return Some(path.join(suffix));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // A dangling symlink is not a directory we can safely create.
                if ancestor.symlink_metadata().is_ok() {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
    None
}

pub(super) fn read(
    path: &Path,
    parse: fn(&str) -> Result<Config, ConfigError>,
) -> Result<Config, SyncError> {
    let text = fs::read_to_string(path).map_err(|source| SyncError::Io {
        path: path.to_owned(),
        source,
    })?;
    parse(&text).map_err(|source| SyncError::Config {
        path: path.to_owned(),
        source,
    })
}

pub(super) fn load(
    root: &Path,
    user_dir: Option<&Path>,
    parse: fn(&str) -> Result<Config, ConfigError>,
) -> Result<Config, SyncError> {
    let mut project = read(&root.join(ARDENT_DIR).join(config::FILE_NAME), parse)?;
    if let Some(dir) = user_dir {
        let path = dir.join(config::FILE_NAME);
        match read(&path, parse) {
            Ok(user) => {
                project_patterns(root, &user)
                    .map_err(|source| SyncError::Pattern { path, source })?;
                project.inherit(user);
            }
            Err(SyncError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(project)
}
