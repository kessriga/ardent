use std::fs::{self, File};
use std::io;
use std::path::Path;

/// Create a cache or recovery directory with its parent's sharing policy.
/// An existing directory returns `AlreadyExists` and retains its permissions.
#[cfg(not(unix))]
pub fn create_shared_dir(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

/// Create a directory with its parent's sharing policy before it becomes visible at the requested path.
#[cfg(unix)]
pub fn create_shared_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    check_absent(path)?;
    let parent = parent_dir(path)?;
    let stage = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o777))
        .tempdir_in(parent)?;
    if !inherit_access(path, &File::open(stage.path())?)? {
        return fs::create_dir(path);
    }
    publish_directory(stage.path(), path, parent)?;
    let _ = stage.keep();
    Ok(())
}

/// Open the project lock for writing, preserving its inode and any existing access rules.
#[cfg(not(unix))]
pub fn open_lock(path: &Path) -> io::Result<File> {
    File::options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
}

/// Open the existing project lock or publish a fully accessible lock without replacing another creator's inode.
#[cfg(unix)]
pub fn open_lock(path: &Path) -> io::Result<File> {
    use rustix::fs::OFlags;
    use std::os::unix::fs::OpenOptionsExt;
    let open = || {
        File::options()
            .write(true)
            .custom_flags(OFlags::NOFOLLOW.bits() as i32)
            .open(path)
    };
    match open() {
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        existing => return existing,
    }
    // Create beside the lock so the filesystem supplies its parent's inherited access rules.
    let parent = parent_dir(path)?;
    let staged = super::create_temp_file(parent)?;
    let needs_setup = inherit_access(path, staged.as_file())?;
    if !needs_setup {
        // Native creation already supplies all required permissions, including the inherited ACL.
        return match File::options().write(true).create_new(true).open(path) {
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => open(),
            result => result,
        };
    }
    match staged.persist_noclobber(path) {
        Ok(file) => Ok(file),
        Err(err) if err.error.kind() == io::ErrorKind::AlreadyExists => open(),
        Err(err) if err.error.kind() == io::ErrorKind::Unsupported => {
            // Some Unix filesystems support hard links but reject exclusive rename.
            match fs::hard_link(err.file.path(), path) {
                Ok(()) => Ok(err.file.into_file()),
                Err(race) if race.kind() == io::ErrorKind::AlreadyExists => open(),
                Err(link) => Err(link),
            }
        }
        Err(err) => Err(err.error),
    }
}

#[cfg(unix)]
fn parent_dir(path: &Path) -> io::Result<&Path> {
    path.parent()
        .ok_or_else(|| io::Error::other("missing parent directory"))
}

#[cfg(unix)]
fn check_absent(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(io::ErrorKind::AlreadyExists.into()),
        Err(err) => Err(err),
    }
}

#[cfg(unix)]
fn publish_directory(source: &Path, destination: &Path, parent: &Path) -> io::Result<()> {
    #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
    {
        use rustix::fs::{self as posix, CWD, RenameFlags};
        use rustix::io::Errno;
        match posix::renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE) {
            Ok(()) => return Ok(()),
            Err(Errno::NOTSUP | Errno::NOSYS | Errno::INVAL) => {}
            Err(err) => return Err(err.into()),
        }
    }
    // Older filesystems without exclusive rename serialize first-time creators on the parent.
    let guard = File::open(parent)?;
    guard.lock()?;
    check_absent(destination)?;
    fs::rename(source, destination)
}

#[cfg(unix)]
fn inherit_access(path: &Path, file: &File) -> io::Result<bool> {
    use rustix::fs::{self as posix, Gid};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let parent = parent_dir(path)?;
    let policy = fs::symlink_metadata(parent)?;
    let meta = file.metadata()?;
    let mask = if meta.is_dir() { 0o2777 } else { 0o666 };
    let mode = policy.mode() & mask;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let mode = {
        use rustix::io::Errno;
        match posix::getxattr_query(parent, "system.posix_acl_default") {
            // The kernel already applied the parent's default ACL to the new entry.
            Ok(_) => (mode & !0o777) | (meta.mode() & 0o777),
            Err(Errno::NODATA | Errno::NOTSUP) => mode,
            Err(err) => return Err(err.into()),
        }
    };
    if meta.gid() == policy.gid() && meta.mode() & 0o7777 == mode {
        return Ok(false);
    }
    if meta.gid() != policy.gid() {
        posix::fchown(file, None, Some(Gid::from_raw(policy.gid())))?;
    }
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    Ok(true)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::sync::Barrier;
    use std::thread;

    #[test]
    fn concurrent_creators_share_one_lock_and_preserve_existing_rules() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("lock");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o770))?;
        let start = Barrier::new(8);
        let inodes = thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        start.wait();
                        Ok::<_, io::Error>(open_lock(&path)?.metadata()?.ino())
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(|thread| thread.join().expect("creator"))
                .collect::<io::Result<Vec<_>>>()
        })?;
        assert!(inodes.iter().all(|inode| *inode == inodes[0]));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        let reopened = open_lock(&path)?.metadata()?;
        assert_eq!(reopened.ino(), inodes[0]);
        assert_eq!(reopened.mode() & 0o777, 0o600);
        Ok(())
    }
}
