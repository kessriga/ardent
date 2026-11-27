use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::staging::Stage;
use ardent_core::layout::{self, ARDENT_DIR, LOCK_FILE};
use ardent_format::{FormatPlugin, InstalledPackages, patterns};

use crate::release::{self, RELEASE, ReleasePlugin};
use crate::sync;

mod transaction;
pub(crate) use transaction::is_journal;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
use crate::windows_access as windows;
#[cfg(all(test, windows))]
mod windows_tests;

type Result<T> = std::result::Result<T, String>;

fn path_error((path, error): (PathBuf, io::Error)) -> String {
    format!("{}: {error}", path.display())
}

fn package_error(name: &str, error: impl fmt::Display) -> String {
    format!("cannot update {name}: {error}")
}

#[derive(Default)]
pub(super) struct Report {
    packages: Vec<&'static ReleasePlugin>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pending: bool,
}

impl Report {
    pub fn names(&self) -> Option<String> {
        (!self.packages.is_empty()).then(|| {
            self.packages
                .iter()
                .map(|plugin| plugin.name)
                .collect::<Vec<_>>()
                .join(", ")
        })
    }
}

fn inspect(ardent: &Path, release: &Path) -> Result<Report> {
    let installed = InstalledPackages::discover(ardent).map_err(|e| e.to_string())?;
    let mut report = Report::default();
    for plugin in RELEASE {
        match transaction::pending(ardent, plugin.name) {
            Ok(pending) => report.pending |= pending,
            Err(err) => {
                report.errors.push(package_error(plugin.name, err));
                continue;
            }
        }
        let changed = installed
            .contains_format(plugin.name)
            .map_err(|e| e.to_string())
            .and_then(|present| {
                if present {
                    plugin.differs(release, ardent).map_err(path_error)
                } else {
                    Ok(false)
                }
            });
        match changed {
            Ok(true) => report.packages.push(plugin),
            Ok(false) => {}
            Err(error) => report.errors.push(package_error(plugin.name, error)),
        }
    }
    Ok(report)
}

fn validate_project(root: &Path, ardent: &Path) -> Result<()> {
    sync::validate_config(root).map_err(|err| err.to_string())?;
    let installed = InstalledPackages::discover(ardent).map_err(|err| err.to_string())?;
    let errors = installed.check_other_manifests(|name| {
        RELEASE.iter().any(|plugin| plugin.name == name) || is_journal(ardent, name)
    });
    if !errors.is_empty() {
        return Err(errors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"));
    }
    Ok(())
}

/// Compare under a shared lock when the project already has one. A fresh project needs no cache.
fn check(ardent: &Path, inspect: impl Fn() -> Result<Report>) -> Result<Report> {
    let path = ardent.join(LOCK_FILE);
    let existing = || match fs::File::open(&path) {
        Ok(file) => Ok(Some(file)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(path_error((path.clone(), err))),
    };
    let lock = match existing()? {
        Some(file) => file,
        None => {
            let initial = inspect();
            // A writer may have created the lock during the first comparison.
            let Some(file) = existing()? else {
                return initial;
            };
            file
        }
    };
    lock.lock_shared().map_err(|e| path_error((path, e)))?;
    inspect()
}

pub(super) fn available(root: &Path) -> Result<Report> {
    let ardent = root.join(ARDENT_DIR);
    let release = release::release_dir().map_err(path_error)?;
    check(&ardent, || inspect(&ardent, &release))
}

pub(super) fn run(working_dir: &Path) -> Result<Report> {
    let root = sync::find_root(working_dir).map_err(|e| e.to_string())?;
    let release = release::release_dir().map_err(path_error)?;
    apply(&root, &release, publish)
}

fn apply(
    root: &Path,
    release: &Path,
    publish: impl Fn(&Path, &Path, &mut Vec<String>) -> io::Result<()>,
) -> Result<Report> {
    let ardent = root.join(ARDENT_DIR);
    if !fs::symlink_metadata(&ardent)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        return Err(format!("{} must be a directory", ardent.display()));
    }
    let mut initial = check(&ardent, || {
        validate_project(root, &ardent)?;
        inspect(&ardent, release)
    })?;
    if initial.packages.is_empty() && !initial.pending {
        return Ok(initial);
    }
    let _lock = match sync::lock(&ardent) {
        Ok(lock) => lock,
        Err(err) => {
            for plugin in std::mem::take(&mut initial.packages) {
                initial.errors.push(package_error(plugin.name, &err));
            }
            if initial.pending {
                initial
                    .errors
                    .push(format!("cannot recover plugin update: {err}"));
            }
            return Ok(initial);
        }
    };
    validate_project(root, &ardent)?;
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let blocked = recover(&ardent, &mut warnings, &mut errors);
    let mut report = inspect(&ardent, release)?;
    report.warnings = warnings;
    report.errors.extend(errors);
    for plugin in std::mem::take(&mut report.packages) {
        if blocked.contains(&plugin.name) {
            continue;
        }
        match replace(plugin, root, release, &publish, &mut report.warnings) {
            Ok(()) => report.packages.push(plugin),
            Err(e) => report.errors.push(package_error(plugin.name, e)),
        }
    }
    Ok(report)
}

fn replace(
    plugin: &ReleasePlugin,
    root: &Path,
    release: &Path,
    publish: &impl Fn(&Path, &Path, &mut Vec<String>) -> io::Result<()>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let ardent = root.join(ARDENT_DIR);
    let tmp = layout::create_dir(&ardent, "cache/tmp").map_err(|e| e.to_string())?;
    let stage = Stage::new(&tmp).map_err(|e| e.to_string())?;
    let result = stage_candidate(plugin, root, release, stage.path()).and_then(|candidate| {
        let installed = ardent.join("plugins").join(plugin.name);
        #[cfg(unix)]
        unix::preserve_access(&candidate, &installed).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        windows::prepare_access(&candidate, &installed).map_err(|e| e.to_string())?;
        publish(&candidate, &installed, warnings).map_err(|e| e.to_string())
    });
    close_stage(stage, plugin.name, warnings);
    result
}

fn close_stage(stage: Stage, name: &str, warnings: &mut Vec<String>) {
    let path = stage.outer_path().to_owned();
    if let Err(err) = stage.close() {
        warnings.push(format!(
            "could not clean up staging files for {name} at {}: {err}",
            path.display()
        ));
    }
}

pub(crate) fn recover(
    ardent: &Path,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) -> HashSet<&'static str> {
    let mut blocked = HashSet::new();
    for plugin in RELEASE {
        let result = transaction::pending(ardent, plugin.name).and_then(|pending| {
            if pending {
                transaction::recover(ardent, plugin, warnings)?;
            }
            Ok(())
        });
        if let Err(err) = result {
            errors.push(format!("cannot recover {}: {err}", plugin.name));
            blocked.insert(plugin.name);
        }
    }
    blocked
}

fn stage_candidate(
    plugin: &ReleasePlugin,
    root: &Path,
    release: &Path,
    stage: &Path,
) -> Result<PathBuf> {
    let ardent = root.join(ARDENT_DIR);
    plugin
        .install(release, &ardent, stage)
        .map_err(path_error)?;
    let format = FormatPlugin::load(stage, plugin.name)
        .map_err(|e| e.to_string())?
        .ok_or("missing staged package")?;
    patterns::compile_defaults(root, &format.default_files, &format.default_prefixes)
        .map_err(|e| e.to_string())?;
    Ok(stage.join("plugins").join(plugin.name))
}

fn publish(candidate: &Path, installed: &Path, warnings: &mut Vec<String>) -> io::Result<()> {
    #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
    {
        use rustix::fs::{self, CWD, RenameFlags};
        use rustix::io::Errno;
        match fs::renameat_with(CWD, candidate, CWD, installed, RenameFlags::EXCHANGE) {
            Ok(()) => return Ok(()),
            Err(Errno::NOTSUP | Errno::NOSYS | Errno::INVAL) => {}
            Err(err) => return Err(err.into()),
        }
    }
    transaction::portable(candidate, installed, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::TryLockError;

    #[test]
    fn publication_holds_the_project_lock_and_a_failed_rename_preserves_the_package()
    -> io::Result<()> {
        let project = tempfile::tempdir()?;
        let root = project.path();
        let ardent = root.join(ARDENT_DIR);
        fs::create_dir(&ardent)?;
        fs::write(
            ardent.join("config.yaml"),
            "schema: 1\nformats: {}\nviews: {}\nexclude: []\n",
        )?;
        let release = release::release_dir().map_err(|(_, e)| e)?;
        let plugin = &RELEASE[0];
        plugin
            .install(&release, &ardent, &ardent)
            .map_err(|(_, e)| e)?;
        let installed = ardent.join("plugins").join(plugin.name);
        fs::write(installed.join("custom"), "local")?;
        let called = std::cell::Cell::new(false);
        let report = apply(root, &release, |candidate, target, warnings| {
            called.set(true);
            let lock = fs::File::open(ardent.join(LOCK_FILE))?;
            assert!(matches!(
                lock.try_lock_shared(),
                Err(TryLockError::WouldBlock)
            ));
            let marker = transaction::start(target, release::package_digest(candidate)?)?;
            fs::remove_dir_all(candidate)?;
            transaction::publish(candidate, target, &marker, warnings)
        })
        .map_err(io::Error::other)?;
        assert!(called.get());
        assert!(report.names().is_none());
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("cannot update markdown"))
        );
        assert_eq!(fs::read(installed.join("custom"))?, b"local");
        assert!(
            FormatPlugin::load(&ardent, plugin.name)
                .map_err(io::Error::other)?
                .is_some()
        );
        Ok(())
    }

    #[test]
    fn portable_publication_preserves_unrelated_files() -> io::Result<()> {
        let project = tempfile::tempdir()?;
        let installed = project.path().join("markdown");
        let stage = tempfile::tempdir_in(project.path())?;
        let candidate = stage.path().join("markdown");
        fs::create_dir(&installed)?;
        fs::create_dir(&candidate)?;
        fs::write(installed.join("old"), "old")?;
        fs::write(candidate.join("new"), "new")?;
        let collision = project.path().join(".ardent-update-markdown");
        fs::create_dir(&collision)?;
        fs::write(collision.join("local"), "mine")?;
        let mut warnings = Vec::new();
        assert_eq!(
            transaction::portable(&candidate, &installed, &mut warnings)
                .expect_err("collision")
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(collision.join("local"))?, b"mine");
        assert_eq!(fs::read(installed.join("old"))?, b"old");
        fs::rename(&collision, project.path().join("mine"))?;
        transaction::portable(&candidate, &installed, &mut warnings)?;
        assert!(warnings.is_empty());
        assert_eq!(fs::read(installed.join("new"))?, b"new");
        assert!(!installed.join("old").try_exists()?);
        assert!(!collision.try_exists()?);
        Ok(())
    }

    #[test]
    fn cleanup_warning_names_the_directory_left_behind() -> io::Result<()> {
        let parent = tempfile::tempdir()?;
        let stage = Stage::new(parent.path())?;
        let path = stage.outer_path().to_owned();
        #[cfg(unix)]
        {
            fs::rename(&path, parent.path().join("saved"))?;
            fs::write(&path, "occupied")?;
        }
        #[cfg(windows)]
        let _open = windows::pin_directory(stage.path())?;
        let mut warnings = Vec::new();
        close_stage(stage, "markdown", &mut warnings);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains(&path.display().to_string()));
        Ok(())
    }
}
