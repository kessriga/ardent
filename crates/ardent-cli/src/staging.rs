use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// An owner-only directory for candidate packages or their verification records.
pub(crate) struct Stage {
    // Windows must release both delete-sharing pins before TempDir cleans up on drop.
    #[cfg(windows)]
    pins: [std::fs::File; 2],
    outer: TempDir,
    path: PathBuf,
}

impl Stage {
    pub(crate) fn new(tmp: &Path) -> io::Result<Self> {
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        {
            use std::fs::Permissions;
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(Permissions::from_mode(0o700));
        }
        let outer = builder.prefix(".ardent-").tempdir_in(tmp)?;
        #[cfg(target_os = "macos")]
        if !exacl::getfacl(outer.path(), None)?.is_empty() {
            // On macOS an inherited allow rule can grant access beyond mode 0700.
            exacl::setfacl(&[outer.path()], &[], None)?;
        }
        #[cfg(windows)]
        let (path, pins) = {
            let parent = crate::windows_access::pin_directory(outer.path())?;
            let path = outer.path().join("private");
            let pin = crate::windows_access::private_directory(&path)?;
            (path, [parent, pin])
        };
        #[cfg(not(windows))]
        let path = outer.path().to_owned();
        Ok(Self {
            outer,
            path,
            #[cfg(windows)]
            pins,
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn close(self) -> io::Result<()> {
        #[cfg(windows)]
        drop(self.pins);
        self.outer.close()
    }

    pub(crate) fn outer_path(&self) -> &Path {
        self.outer.path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_a_stage_removes_its_contents() -> io::Result<()> {
        let parent = tempfile::tempdir()?;
        let stage = Stage::new(parent.path())?;
        std::fs::write(stage.path().join("candidate"), b"ready")?;
        drop(stage);
        assert_eq!(parent.path().read_dir()?.count(), 0);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn candidate_staging_is_private() -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let parent = tempfile::tempdir()?;
        #[cfg(target_os = "macos")]
        {
            use exacl::{AclEntry, Flag, Perm};
            let rule = AclEntry::allow_group("everyone", Perm::READ, Flag::DIRECTORY_INHERIT);
            exacl::setfacl(&[parent.path()], &[rule], None)?;
        }
        let stage = Stage::new(parent.path())?;
        assert_eq!(stage.path().metadata()?.permissions().mode() & 0o777, 0o700);
        #[cfg(target_os = "macos")]
        assert!(exacl::getfacl(stage.path(), None)?.is_empty());
        Ok(())
    }
}
