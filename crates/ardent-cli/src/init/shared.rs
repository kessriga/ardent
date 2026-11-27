use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ardent_core::layout;
use ardent_format::{InstalledPackages, patterns};

use crate::release::{self, ReleasePlugin};
use crate::staging::Stage;
use crate::xdg;

type Result<T> = std::result::Result<T, (PathBuf, io::Error)>;

pub(super) fn install(
    project: &Path,
    plugins: &[&ReleasePlugin],
    published: &mut Vec<&'static str>,
) -> Result<()> {
    let home = xdg::config_home().ok_or_else(|| {
        (
            PathBuf::from("XDG_CONFIG_HOME"),
            io::Error::other("cannot locate user configuration"),
        )
    })?;
    let user = home.join("ardent");
    let packages = user.join("plugins");
    // Verification records belong to this init, including when reusing a read-only user package.
    let scratch = Stage::new(project).map_err(|err| (project.to_owned(), err))?;
    for plugin in plugins {
        let destination = packages.join(plugin.name);
        let verify = |root: &Path| verify(scratch.path(), root, plugin.name);
        let was_missing = matches!(fs::symlink_metadata(&destination), Err(err) if err.kind() == io::ErrorKind::NotFound);
        if !was_missing {
            require_package(verify(&user)?, &destination)?;
            continue;
        }
        fs::create_dir_all(&home).map_err(|err| (home.clone(), err))?;
        layout::create_dir(&home, "ardent/plugins").map_err(|err| (packages.clone(), err))?;
        match verify(&user) {
            Ok(true) => continue,
            Ok(false) => {}
            // Discovery may list the directory just before another init publishes this package.
            Err(_) if destination.exists() => {
                require_package(verify(&user)?, &destination)?;
                continue;
            }
            Err(err) => return Err(err),
        }
        let stage = Stage::new(&packages).map_err(|err| (packages.clone(), err))?;
        plugin.install(&release::release_dir()?, project, stage.path())?;
        let candidate = stage.path().join("plugins").join(plugin.name);
        require_package(verify(stage.path())?, &candidate)?;
        match layout::publish_directory(&candidate, &destination) {
            Ok(()) => published.push(plugin.name),
            Err(_) if fs::symlink_metadata(&destination).is_ok() => {
                require_package(verify(&user)?, &destination)?;
            }
            Err(err) => return Err((destination, err)),
        }
    }
    Ok(())
}

fn require_package(found: bool, path: &Path) -> Result<()> {
    if found {
        Ok(())
    } else {
        Err((path.to_owned(), io::ErrorKind::NotFound.into()))
    }
}

fn verify(scratch: &Path, installation: &Path, name: &str) -> Result<bool> {
    let path = installation.join("plugins").join(name);
    let format = InstalledPackages::for_sync(scratch, Some(installation), None)
        .and_then(|packages| packages.load_format(name))
        .map_err(|err| (path.clone(), io::Error::other(err)))?;
    let Some(format) = format else {
        return Ok(false);
    };
    patterns::compile_defaults(
        Path::new("."),
        &format.default_files,
        &format.default_prefixes,
    )
    .map_err(|err| (path, io::Error::other(err)))?;
    Ok(true)
}
