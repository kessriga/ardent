mod acl;

use std::fs;
use std::io;
use std::os::unix::fs::{self as unix, MetadataExt, PermissionsExt};
use std::path::Path;

#[derive(Clone, Copy)]
struct Access {
    owner: Option<u32>,
    group: u32,
    mode: u32,
}

impl Access {
    fn existing(meta: &fs::Metadata) -> Self {
        // A release replaces the contents. Never transfer set-user-ID or set-group-ID to a new file.
        let mask = if meta.is_dir() { 0o3777 } else { 0o777 };
        Self {
            owner: Some(meta.uid()),
            group: meta.gid(),
            mode: meta.mode() & mask,
        }
    }

    fn child(self, directory: bool) -> Self {
        let mask = if directory { 0o2777 } else { 0o666 };
        Self {
            owner: None,
            mode: self.mode & mask,
            group: self.group,
        }
    }

    fn apply(self, path: &Path) -> io::Result<()> {
        let meta = fs::symlink_metadata(path)?;
        let owner = self.owner.filter(|uid| *uid != meta.uid());
        let group = (self.group != meta.gid()).then_some(self.group);
        if owner.is_some() || group.is_some() {
            unix::chown(path, owner, group).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("cannot preserve ownership of {}: {err}", path.display()),
                )
            })?;
        }
        fs::set_permissions(path, fs::Permissions::from_mode(self.mode))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_keeps_the_original_owner_or_fails_before_changing_access() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let staged = dir.path().join("staged");
        fs::File::create_new(&staged)?;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o600))?;
        let creator = staged.metadata()?;
        // A root-owned entry supplies a different original owner in an unprivileged test run.
        let original = fs::metadata("/")?;
        let mut access = Access::existing(&original);
        access.group = creator.gid();
        access.mode = 0o644;
        match access.apply(&staged) {
            Ok(()) => assert_eq!(staged.metadata()?.uid(), original.uid()),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
                assert_eq!(staged.metadata()?.mode() & 0o777, 0o600);
            }
        }
        let added = dir.path().join("added");
        fs::File::create_new(&added)?;
        access.child(false).apply(&added)?;
        assert_eq!(added.metadata()?.uid(), creator.uid());
        Ok(())
    }
}
