use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use same_file::Handle;

use crate::tracked::TrackedFile;

/// The immediate child names from tracked source paths, with the spelling returned by the source walk.
#[derive(Default)]
pub(super) struct CacheNames<'a>(HashMap<&'a str, HashSet<&'a str>>);

impl<'a> CacheNames<'a> {
    pub(super) fn from_files(files: &'a [TrackedFile]) -> Self {
        let mut dirs = Self::default();
        for file in files {
            let mut path = file.path.as_str();
            loop {
                let (parent, name) = path.rsplit_once('/').unwrap_or(("", path));
                if !dirs.0.entry(parent).or_default().insert(name) || parent.is_empty() {
                    break;
                }
                path = parent;
            }
        }
        dirs
    }

    pub(super) fn has_children(&self, source_path: &str) -> bool {
        self.0.contains_key(source_path)
    }

    pub(super) fn align(
        &mut self,
        dir: &Path,
        source_path: &str,
        entries: &mut [(fs::FileType, PathBuf)],
    ) -> io::Result<()> {
        let Some(mut missing) = self.0.remove(source_path) else {
            return Ok(());
        };
        let mut candidates = Vec::new();
        for (kind, path) in entries {
            if kind.is_dir()
                && !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| missing.remove(n))
            {
                candidates.push(path);
            }
        }
        for name in missing {
            if candidates.is_empty() {
                break;
            }
            let desired = dir.join(name);
            match fs::symlink_metadata(&desired) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => continue,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err),
            }
            // An existing directory with a different spelling aliases the tracked path on this volume.
            let desired_dir = Handle::from_path(&desired)?;
            let index = candidates
                .iter()
                .position(|path| Handle::from_path(path).is_ok_and(|actual| actual == desired_dir))
                .ok_or_else(|| io::Error::other(format!("cannot locate {}", desired.display())))?;
            let actual = candidates.swap_remove(index);
            fs::rename(&*actual, &desired)?;
            *actual = desired;
        }
        Ok(())
    }
}
