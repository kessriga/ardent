mod acl;

use std::fs;
use std::io;
use std::os::unix::fs::{self as unix, MetadataExt, PermissionsExt};
use std::path::Path;

#[derive(Clone, Copy)]
struct Access {
    group: u32,
    mode: u32,
}

impl Access {
    fn existing(meta: &fs::Metadata) -> Self {
        // A release replaces the contents. Never transfer set-user-ID or set-group-ID to a new file.
        let mask = if meta.is_dir() { 0o3777 } else { 0o777 };
        Self {
            group: meta.gid(),
            mode: meta.mode() & mask,
        }
    }

    fn child(self, directory: bool) -> Self {
        let mask = if directory { 0o2777 } else { 0o666 };
        Self {
            mode: self.mode & mask,
            ..self
        }
    }

    fn apply(self, path: &Path) -> io::Result<()> {
        if fs::symlink_metadata(path)?.gid() != self.group {
            unix::chown(path, None, Some(self.group))?;
        }
        fs::set_permissions(path, fs::Permissions::from_mode(self.mode))
    }
}

/// A collaborator who can replace packages must also be able to recover an interrupted rename.
pub(super) fn share_journal_directory(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("missing journal parent"))?;
    let access = Access::existing(&fs::symlink_metadata(parent)?).child(true);
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let access = Access {
        mode: acl::inherited_directory_mode(path, parent, access.mode)?,
        ..access
    };
    access.apply(path)
}

/// Match existing paths and let added entries inherit the containing directory's sharing policy.
pub(super) fn preserve_access(candidate: &Path, installed: &Path) -> io::Result<()> {
    let scratch = candidate
        .parent()
        .ok_or_else(|| io::Error::other("missing staging directory"))?;
    let mut root = Access::existing(&fs::symlink_metadata(installed)?);
    root.mode = acl::copy(Some(installed), candidate, scratch, scratch, root.mode)?;
    let mut dirs = vec![(candidate.to_owned(), Some(installed.to_owned()), root)];
    let mut directory_modes = Vec::new();
    while let Some((dir, old_dir, access)) = dirs.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let is_dir = entry.file_type()?.is_dir();
            let old = old_dir.as_ref().map(|path| path.join(entry.file_name()));
            let meta = match old.as_ref().map(fs::symlink_metadata).transpose() {
                Ok(meta) => meta.filter(|m| if is_dir { m.is_dir() } else { m.is_file() }),
                Err(err) if err.kind() == io::ErrorKind::NotFound => None,
                Err(err) => return Err(err),
            };
            let old = old.filter(|_| meta.is_some());
            let path = entry.path();
            let mut child = meta
                .as_ref()
                .map_or_else(|| access.child(is_dir), Access::existing);
            child.mode = acl::copy(old.as_deref(), &path, &dir, scratch, child.mode)?;
            if is_dir {
                dirs.push((path, old, child));
            } else {
                child.apply(&path)?;
            }
        }
        directory_modes.push((dir, access));
    }
    // A read-only directory can still contain files whose permissions need updating.
    for (path, access) in directory_modes.into_iter().rev() {
        access.apply(&path)?;
    }
    Ok(())
}
