use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ardent_core::layout::{self, ARDENT_DIR, LOCK_FILE};
use ardent_format::{FormatPlugin, InstalledPackages, patterns};
use tempfile::TempDir;

use crate::release::{self, RELEASE, ReleasePlugin};
use crate::sync;

type Result<T> = std::result::Result<T, String>;

fn path_error((path, error): (PathBuf, io::Error)) -> String {
    format!("{}: {error}", path.display())
}

fn package_error(name: &str, error: impl fmt::Display) -> String {
    format!("cannot update {name}: {error}")
}

fn changed(ardent: &Path, release: &Path) -> Result<Vec<&'static ReleasePlugin>> {
    let installed = InstalledPackages::discover(ardent).map_err(|e| e.to_string())?;
    let mut changed = Vec::new();
    for plugin in RELEASE {
        if !installed
            .contains_format(plugin.name)
            .map_err(|e| package_error(plugin.name, e))?
        {
            continue;
        }
        if plugin
            .differs(release, ardent)
            .map_err(|e| package_error(plugin.name, path_error(e)))?
        {
            changed.push(plugin);
        }
    }
    Ok(changed)
}

/// Called by the browser worker after its first sync and after a project plugin changes.
pub(super) fn notice(root: &Path) -> Result<Option<String>> {
    let ardent = root.join(ARDENT_DIR);
    let release = release::release_dir().map_err(path_error)?;
    let _lock = sync::lock(&ardent).map_err(|e| e.to_string())?;
    let names: Vec<_> = changed(&ardent, &release)?.iter().map(|p| p.name).collect();
    Ok((!names.is_empty())
        .then(|| format!("Plugin updates: {}. Run ardent update", names.join(", "))))
}

pub(super) fn run(working_dir: &Path) -> Result<Vec<&'static str>> {
    let root = sync::find_root(working_dir).map_err(|e| e.to_string())?;
    let ardent = root.join(ARDENT_DIR);
    if !fs::symlink_metadata(&ardent)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err(format!("{} must be a directory", ardent.display()));
    }
    let release = release::release_dir().map_err(path_error)?;
    // A fresh, current project should stay byte-for-byte as init created it, without a cache.
    if !ardent
        .join(LOCK_FILE)
        .try_exists()
        .map_err(|e| e.to_string())?
        && changed(&ardent, &release)?.is_empty()
    {
        return Ok(Vec::new());
    }
    let _lock = sync::lock(&ardent).map_err(|e| e.to_string())?;
    let mut updated = Vec::new();
    for plugin in changed(&ardent, &release)? {
        replace(plugin, &root, &release).map_err(|e| package_error(plugin.name, e))?;
        updated.push(plugin.name);
    }
    Ok(updated)
}

fn replace(plugin: &ReleasePlugin, root: &Path, release: &Path) -> Result<()> {
    let ardent = root.join(ARDENT_DIR);
    let tmp = layout::create_dir(&ardent, "cache/tmp").map_err(|e| e.to_string())?;
    let stage = TempDir::new_in(tmp).map_err(|e| e.to_string())?;
    plugin
        .install(release, &ardent, stage.path())
        .map_err(path_error)?;
    let format = FormatPlugin::load(stage.path(), plugin.name)
        .map_err(|e| e.to_string())?
        .ok_or("missing staged package")?;
    patterns::compile_defaults(root, &format.default_files, &format.default_prefixes)
        .map_err(|e| e.to_string())?;
    let relative = Path::new("plugins").join(plugin.name);
    publish(stage, &relative, &ardent.join(&relative)).map_err(|e| e.to_string())
}

fn publish(stage: TempDir, relative: &Path, installed: &Path) -> io::Result<()> {
    let candidate = stage.path().join(relative);
    #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
    {
        use rustix::fs::{self, CWD, RenameFlags};
        use rustix::io::Errno;
        match fs::renameat_with(CWD, &candidate, CWD, installed, RenameFlags::EXCHANGE) {
            Ok(()) => return Ok(()),
            Err(Errno::NOTSUP | Errno::NOSYS | Errno::INVAL) => {}
            Err(err) => return Err(err.into()),
        }
    }
    publish_with_backup(stage, &candidate, installed)
}

fn publish_with_backup(stage: TempDir, candidate: &Path, installed: &Path) -> io::Result<()> {
    let previous = stage.path().join("previous");
    fs::rename(installed, &previous)?;
    if let Err(error) = fs::rename(candidate, installed) {
        if let Err(restore) = fs::rename(&previous, installed) {
            let _ = stage.keep();
            return Err(io::Error::other(format!(
                "{error}; previous package retained at {}: {restore}",
                previous.display()
            )));
        }
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_portable_rename_restores_the_original_directory() -> io::Result<()> {
        let project = tempfile::tempdir()?;
        let installed = project.path().join("markdown");
        fs::create_dir(&installed)?;
        fs::write(installed.join("old"), b"original")?;
        let stage = TempDir::new_in(project.path())?;
        let missing = stage.path().join("missing");
        assert!(publish_with_backup(stage, &missing, &installed).is_err());
        assert_eq!(fs::read(installed.join("old"))?, b"original");
        Ok(())
    }
}
