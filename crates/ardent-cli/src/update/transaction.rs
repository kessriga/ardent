use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::release::{self, ReleasePlugin};
use crate::staging::Stage;
use ardent_core::layout;

const JOURNAL_PREFIX: &str = ".ardent-update-";
const MARKER: &str = "ardent-plugin-update-v1";
const CANDIDATE: &str = "candidate-";
const PREVIOUS: &str = "previous";

fn journal(installed: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(JOURNAL_PREFIX);
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

fn has_marker(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => directory(&path.join(MARKER)),
        Ok(_) => Ok(false),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

/// A marked backup can retain unrelated files after its pending update has been recovered.
pub(crate) fn is_journal(ardent: &Path, name: &str) -> bool {
    name.strip_prefix(JOURNAL_PREFIX)
        .is_some_and(|suffix| release::RELEASE.iter().any(|plugin| plugin.name == suffix))
        && has_marker(&ardent.join("plugins").join(name)).unwrap_or(false)
}

pub(super) fn pending(ardent: &Path, name: &str) -> io::Result<bool> {
    let plugins = ardent.join("plugins");
    if !directory(ardent)? || !directory(&plugins)? {
        return Ok(false);
    }
    let path = journal(&plugins.join(name));
    if !has_marker(&path)? {
        return Ok(false);
    }
    let marker = path.join(MARKER);
    Ok(directory(&marker.join(PREVIOUS))?
        || receipt(&marker)?.is_some()
        || (empty(&marker)? && only_marker(&path)?))
}

fn empty(dir: &Path) -> io::Result<bool> {
    Ok(fs::read_dir(dir)?.next().transpose()?.is_none())
}

fn only_marker(path: &Path) -> io::Result<bool> {
    for entry in fs::read_dir(path)? {
        if entry?.file_name() != MARKER {
            return Ok(false);
        }
    }
    Ok(true)
}

// An empty directory records the digest atomically, including on filesystems without hard links.
fn receipt(marker: &Path) -> io::Result<Option<(PathBuf, blake3::Hash)>> {
    let mut receipt = None;
    for entry in fs::read_dir(marker)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(hex) = name.to_str().and_then(|name| name.strip_prefix(CANDIDATE)) else {
            continue;
        };
        let Ok(digest) = blake3::Hash::from_hex(hex) else {
            continue;
        };
        if !entry.file_type()?.is_dir() || receipt.is_some() {
            return Err(io::Error::other(format!(
                "ambiguous update receipt in {}",
                marker.display()
            )));
        }
        receipt = Some((entry.path(), digest));
    }
    Ok(receipt)
}

pub(super) fn start(installed: &Path, digest: blake3::Hash) -> io::Result<PathBuf> {
    let path = journal(installed);
    let marker = path.join(MARKER);
    match layout::create_shared_dir(&path) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            if !directory(&path)? || (!directory(&marker)? && !empty(&path)?) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} already exists", path.display()),
                ));
            }
        }
        Err(err) => return Err(err),
    }
    if !directory(&marker)? {
        layout::create_shared_dir(&marker)?;
    }
    if directory(&marker.join(PREVIOUS))? {
        return Err(io::Error::other(format!(
            "previous package still in {}",
            marker.display()
        )));
    }
    if receipt(&marker)?.is_some() {
        return Err(io::Error::other(format!(
            "unfinished update at {}",
            marker.display()
        )));
    }
    fs::create_dir(marker.join(format!("{CANDIDATE}{digest}")))?;
    Ok(marker)
}

// Remove only the entries the updater owns. Directory removal also rejects newly arrived files.
fn clean_marker(marker: &Path, installed: &Path) -> io::Result<()> {
    if let Some((receipt, _)) = receipt(marker)? {
        fs::remove_dir(receipt)?;
    }
    let path = journal(installed);
    if !empty(marker)? || !only_marker(&path)? {
        return Err(io::Error::other("unrelated files remain"));
    }
    fs::remove_dir(marker)?;
    fs::remove_dir(path)
}

fn warn_cleanup(installed: &Path, result: io::Result<()>, warnings: &mut Vec<String>) {
    if let Err(err) = result {
        warnings.push(format!(
            "could not clean up {}: {err}",
            journal(installed).display()
        ));
    }
}

pub(super) fn portable(
    candidate: &Path,
    installed: &Path,
    warnings: &mut Vec<String>,
) -> io::Result<()> {
    let marker = start(installed, release::package_digest(candidate)?)?;
    publish(candidate, installed, &marker, warnings)
}

pub(super) fn publish(
    candidate: &Path,
    installed: &Path,
    marker: &Path,
    warnings: &mut Vec<String>,
) -> io::Result<()> {
    let previous = marker.join(PREVIOUS);
    if let Err(err) = fs::rename(installed, &previous) {
        warn_cleanup(installed, clean_marker(marker, installed), warnings);
        return Err(err);
    }
    if let Err(err) = fs::rename(candidate, installed) {
        fs::rename(&previous, installed).map_err(|restore| {
            io::Error::other(format!(
                "{err}; original saved at {}: {restore}",
                previous.display()
            ))
        })?;
        warn_cleanup(installed, clean_marker(marker, installed), warnings);
        return Err(err);
    }
    let retired = fs::rename(previous, candidate).and_then(|()| clean_marker(marker, installed));
    warn_cleanup(installed, retired, warnings);
    Ok(())
}

fn matches_candidate(
    ardent: &Path,
    plugin: &ReleasePlugin,
    installed: &Path,
    marker: &Path,
) -> io::Result<bool> {
    if let Some((_, expected)) = receipt(marker)? {
        return Ok(release::package_digest(installed)? == expected);
    }
    // Earlier updates saved only the previous directory. Compare with a known complete bundle.
    let release = release::release_dir().map_err(|(_, err)| err)?;
    plugin
        .differs(&release, ardent)
        .map(|changed| !changed)
        .map_err(|(_, err)| err)
}

/// Recover one interrupted update while holding the exclusive project lock.
pub(super) fn recover(
    ardent: &Path,
    plugin: &ReleasePlugin,
    warnings: &mut Vec<String>,
) -> io::Result<()> {
    let installed = ardent.join("plugins").join(plugin.name);
    let marker = journal(&installed).join(MARKER);
    let previous = marker.join(PREVIOUS);
    let has_previous = directory(&previous)?;
    let recovery_error = |err: io::Error| {
        if !has_previous {
            return err;
        }
        io::Error::new(
            err.kind(),
            format!(
                "{err}; original saved at {}. To restore it, move {} aside and run ardent sync",
                previous.display(),
                installed.display(),
            ),
        )
    };
    let has_installed = directory(&installed).map_err(recovery_error)?;
    if !has_installed || (has_previous && empty(&installed).map_err(recovery_error)?) {
        if !has_previous {
            return Err(io::Error::other(format!(
                "missing installed package and backup at {}",
                previous.display()
            )));
        }
        if has_installed {
            fs::remove_dir(&installed)?;
        }
        fs::rename(&previous, &installed)?;
    } else if has_previous {
        let verified =
            matches_candidate(ardent, plugin, &installed, &marker).map_err(recovery_error)?;
        if !verified {
            return Err(io::Error::other(format!(
                "installed package differs from the replacement; original saved at {}",
                previous.display()
            )));
        }
        #[cfg(windows)]
        // Older Windows updaters published before restoring project permissions.
        super::windows::repair_access(&installed, &previous).map_err(recovery_error)?;
        let retired = (|| {
            let tmp = layout::create_dir(ardent, "cache/tmp")?;
            let stage = Stage::new(&tmp)?;
            let moved = fs::rename(&previous, stage.path().join(PREVIOUS));
            super::close_stage(stage, plugin.name, warnings);
            moved
        })();
        if retired.is_err() {
            warn_cleanup(&installed, retired, warnings);
            return Ok(());
        }
    }
    warn_cleanup(&installed, clean_marker(&marker, &installed), warnings);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::RELEASE;

    #[test]
    fn recovery_requires_the_recorded_package_even_if_the_release_has_changed() -> io::Result<()> {
        let project = tempfile::tempdir()?;
        let ardent = project.path().join(".ardent");
        let plugin = &RELEASE[0];
        let installed = ardent.join("plugins").join(plugin.name);
        fs::create_dir_all(&installed)?;
        fs::write(installed.join("local"), "old")?;
        let candidate = project.path().join("candidate");
        fs::create_dir_all(candidate.join("empty"))?;
        fs::write(candidate.join("notes"), "new")?;

        let marker = start(&installed, release::package_digest(&candidate)?)?;
        let previous = marker.join(PREVIOUS);
        fs::rename(&installed, &previous)?;
        fs::rename(&candidate, &installed)?;
        fs::remove_dir(installed.join("empty"))?;
        let mut warnings = Vec::new();
        assert!(recover(&ardent, plugin, &mut warnings).is_err());
        assert_eq!(fs::read(previous.join("local"))?, b"old");

        fs::create_dir(installed.join("empty"))?;
        fs::write(marker.join("reader-note"), "keep")?;
        recover(&ardent, plugin, &mut warnings)?;
        assert_eq!(fs::read(installed.join("notes"))?, b"new");
        assert!(!previous.try_exists()?);
        assert_eq!(fs::read(marker.join("reader-note"))?, b"keep");
        assert!(!warnings.is_empty());
        assert!(!pending(&ardent, plugin.name)?);

        fs::create_dir(&candidate)?;
        fs::write(candidate.join("latest"), "latest")?;
        portable(&candidate, &installed, &mut warnings)?;
        assert_eq!(fs::read(installed.join("latest"))?, b"latest");
        assert_eq!(fs::read(marker.join("reader-note"))?, b"keep");
        Ok(())
    }
}
