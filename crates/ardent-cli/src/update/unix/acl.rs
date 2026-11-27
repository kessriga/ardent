//! Preserve named access rules in addition to the Unix group and mode bits.
use std::io;
use std::path::Path;

#[cfg(target_os = "macos")]
pub(super) fn copy(
    old: Option<&Path>,
    new: &Path,
    parent: &Path,
    scratch: &Path,
    mode: u32,
) -> io::Result<u32> {
    use exacl::Flag;
    use std::fs;

    let entries = if let Some(old) = old {
        exacl::getfacl(old, None)?
    } else {
        let mut rules = exacl::getfacl(parent, None)?;
        rules.retain(|r| {
            r.flags
                .intersects(Flag::FILE_INHERIT | Flag::DIRECTORY_INHERIT)
        });
        if rules.is_empty() {
            Vec::new()
        } else {
            // Let the filesystem resolve inheritance, including rules limited to one generation.
            let probe = tempfile::tempdir_in(scratch)?;
            for rule in &mut rules {
                rule.flags.insert(Flag::ONLY_INHERIT);
            }
            exacl::setfacl(&[probe.path()], &rules, None)?;
            let child = probe.path().join("child");
            if new.is_dir() {
                fs::create_dir(&child)?;
            } else {
                fs::File::create_new(&child)?;
            }
            let inherited = exacl::getfacl(&child, None)?;
            exacl::setfacl(&[&child], &[], None)?;
            probe.close()?;
            inherited
        }
    };
    if exacl::getfacl(new, None)? != entries {
        exacl::setfacl(&[new], &entries, None)?;
    }
    Ok(mode)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod posix {
    use rustix::fs::{self, XattrFlags};
    use rustix::io::Errno;
    use std::io;
    use std::path::Path;

    pub const ACCESS: &str = "system.posix_acl_access";
    pub const DEFAULT: &str = "system.posix_acl_default";

    pub fn get(path: &Path, key: &str) -> io::Result<Option<Vec<u8>>> {
        let size = match fs::getxattr_query(path, key) {
            Ok(size) => size,
            Err(Errno::NODATA | Errno::NOTSUP) => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let mut data = vec![0; size];
        let len = fs::getxattr(path, key, data.as_mut_slice())?;
        data.truncate(len);
        Ok(Some(data))
    }

    pub fn set(path: &Path, key: &str, value: Option<&[u8]>) -> io::Result<()> {
        if let Some(data) = value {
            fs::setxattr(path, key, data, XattrFlags::empty())?;
        } else if let Err(err) = fs::removexattr(path, key)
            && !matches!(err, Errno::NODATA | Errno::NOTSUP)
        {
            return Err(err.into());
        }
        Ok(())
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn copy(
    old: Option<&Path>,
    new: &Path,
    parent: &Path,
    _scratch: &Path,
    mode: u32,
) -> io::Result<u32> {
    use posix::{ACCESS, DEFAULT};
    let access = match old {
        Some(path) => posix::get(path, ACCESS)?,
        None => posix::get(parent, DEFAULT)?,
    };
    posix::set(new, ACCESS, access.as_deref())?;
    if new.is_dir() {
        let default = posix::get(old.unwrap_or(parent), DEFAULT)?;
        posix::set(new, DEFAULT, default.as_deref())?;
    }
    // Linux inherits the parent's default ACL subject to the requested creation mode.
    if old.is_none() && access.is_some() {
        return Ok(inherited_mode(&new.metadata()?, mode));
    }
    Ok(mode)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn inherited_mode(meta: &std::fs::Metadata, mode: u32) -> u32 {
    use std::os::unix::fs::MetadataExt;
    let creation = if meta.is_dir() { 0o777 } else { 0o666 };
    (mode & !0o777) | (meta.mode() & creation)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "android")))]
pub(super) fn copy(
    _old: Option<&Path>,
    _new: &Path,
    _parent: &Path,
    _scratch: &Path,
    mode: u32,
) -> io::Result<u32> {
    Ok(mode)
}
