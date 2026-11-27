use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::staging_dir;
use ardent_core::layout;

// The marker directory and its contents move together, so every visible journal is recognizable.
const MARKER: &str = "ardent-plugin-update-v1";
const PREVIOUS: &str = "previous";

fn journal(installed: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".ardent-update-");
    name.push(installed.file_name().unwrap_or_default());
    installed.with_file_name(name)
}

fn directory(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(true),
        Ok(_) => Err(io::Error::other(format!(
            "{} must be a directory",
            path.display()
        ))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

pub(super) fn pending(ardent: &Path, name: &str) -> io::Result<bool> {
    let plugins = ardent.join("plugins");
    if !directory(ardent)? || !directory(&plugins)? {
        return Ok(false);
    }
    let path = journal(&plugins.join(name));
    // A name collision alone does not make a third-party directory ours.
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Ok(false),
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    }
    directory(&path.join(MARKER))
}

fn retire(journal: &Path, trash: &Path, warnings: &mut Vec<String>) {
    if let Err(err) = fs::rename(journal, trash) {
        warnings.push(format!("could not clean up {}: {err}", journal.display()));
    }
}

pub(super) fn portable(
    candidate: &Path,
    installed: &Path,
    warnings: &mut Vec<String>,
) -> io::Result<()> {
    let path = journal(installed);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", path.display()),
            ));
        }
        Err(e) => return Err(e),
    }
    let trash = candidate.with_file_name("transaction");
    fs::create_dir(&trash)?;
    #[cfg(windows)]
    super::windows::restore_access(&trash, installed)?;
    fs::create_dir(trash.join(MARKER))?;
    fs::rename(&trash, &path)?;
    let previous = path.join(MARKER).join(PREVIOUS);
    if let Err(err) = fs::rename(installed, &previous) {
        retire(&path, &trash, warnings);
        return Err(err);
    }
    if let Err(err) = fs::rename(candidate, installed) {
        fs::rename(&previous, installed).map_err(|restore| {
            io::Error::other(format!(
                "{err}; original saved at {}: {restore}",
                previous.display()
            ))
        })?;
        retire(&path, &trash, warnings);
        return Err(err);
    }
    #[cfg(windows)]
    if let Err(err) = super::windows::restore_access(installed, &previous) {
        fs::rename(installed, candidate)
            .and_then(|()| fs::rename(&previous, installed))
            .map_err(|restore| {
                io::Error::other(format!(
                    "{err}; original saved at {}: {restore}",
                    previous.display()
                ))
            })?;
        retire(&path, &trash, warnings);
        return Err(err);
    }
    retire(&path, &trash, warnings);
    Ok(())
}

// Called under the exclusive project lock, before installed packages or cached structures are read.
pub(super) fn recover(ardent: &Path, name: &str, warnings: &mut Vec<String>) -> io::Result<()> {
    let installed = ardent.join("plugins").join(name);
    let path = journal(&installed);
    let marker = path.join(MARKER);
    for (dir, expected) in [(&path, MARKER), (&marker, PREVIOUS)] {
        for entry in fs::read_dir(dir)? {
            if entry?.file_name() != expected {
                return Err(io::Error::other(format!(
                    "unexpected files in {}",
                    dir.display()
                )));
            }
        }
    }
    let previous = marker.join(PREVIOUS);
    let has_previous = directory(&previous)?;
    let has_installed = directory(&installed)?;
    if !has_installed {
        if !has_previous {
            return Err(io::Error::other(format!(
                "missing installed package and backup in {}",
                path.display()
            )));
        }
        fs::rename(&previous, &installed)?;
    }
    #[cfg(windows)]
    if has_installed && has_previous {
        super::windows::restore_access(&installed, &previous)?;
    }
    let cleanup = (|| -> io::Result<()> {
        let tmp = layout::create_dir(ardent, "cache/tmp")?;
        let stage = staging_dir(&tmp)?;
        fs::rename(&path, stage.path.join("transaction"))?;
        super::close_stage(stage, name, warnings);
        Ok(())
    })();
    if let Err(err) = cleanup {
        warnings.push(format!("could not clean up {}: {err}", path.display()));
    }
    Ok(())
}
