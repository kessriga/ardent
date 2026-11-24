use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Physical directory names during deferred pruning. A successful scan must leave a path that the next prune keeps.
#[derive(Default)]
pub(super) struct CacheNames(HashMap<PathBuf, HashSet<OsString>>);

impl CacheNames {
    pub(super) fn correct_path(
        &mut self,
        mut parent: PathBuf,
        source_path: &str,
    ) -> io::Result<()> {
        for name in Path::new(source_path).iter() {
            let path = parent.join(name);
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::NotADirectory,
                        format!("{} is not a directory", path.display()),
                    ));
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    // The atomic write creates the remaining directories with the source spelling.
                    if let Some(names) = self.0.get_mut(&parent) {
                        names.insert(name.to_owned());
                    }
                    return Ok(());
                }
                Err(err) => return Err(err),
            }
            self.correct_entry(&parent, name, &path)?;
            parent = path;
        }
        Ok(())
    }

    fn correct_entry(&mut self, parent: &Path, name: &OsStr, path: &Path) -> io::Result<()> {
        let names = match self.0.entry(parent.to_owned()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(
                fs::read_dir(parent)?
                    .map(|entry| entry.map(|entry| entry.file_name()))
                    .collect::<io::Result<_>>()?,
            ),
        };
        if names.contains(name) {
            return Ok(());
        }
        let old = names
            .iter()
            .find(|actual| {
                let candidate = parent.join(actual);
                fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.is_dir())
                    && same_file::is_same_file(&candidate, path).unwrap_or(false)
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("cannot locate cache directory {}", path.display()),
                )
            })?
            .clone();
        fs::rename(parent.join(&old), path)?;
        names.remove(&old);
        names.insert(name.to_owned());
        Ok(())
    }
}
