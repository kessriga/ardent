use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ardent_core::layout;

pub(super) struct ReleasePlugin {
    pub(super) name: &'static str,
    required_files: &'static [&'static str],
}

pub(super) const RELEASE: &[ReleasePlugin] = &[
    ReleasePlugin {
        name: "markdown",
        required_files: &["plugin.yaml", "format.wasm"],
    },
    ReleasePlugin {
        name: "rust",
        required_files: &["plugin.yaml", "format.wasm", "README.md"],
    },
];

pub(super) fn release_dir() -> Result<PathBuf, (PathBuf, io::Error)> {
    let executable =
        env::current_exe().map_err(|err| (PathBuf::from("current executable"), err))?;
    let dir = fs::canonicalize(&executable)
        .map_err(|err| (executable, err))?
        .with_file_name("plugins");
    #[cfg(debug_assertions)]
    if !dir.try_exists().map_err(|err| (dir.clone(), err))? {
        return Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins"));
    }
    Ok(dir)
}

impl ReleasePlugin {
    pub(super) fn install(
        &self,
        source: &Path,
        ardent_dir: &Path,
    ) -> Result<(), (PathBuf, io::Error)> {
        let package = source.join(self.name);
        for file in self.required_files {
            let path = package.join(file);
            if !fs::symlink_metadata(&path)
                .map_err(|err| (path.clone(), err))?
                .is_file()
            {
                return Err(invalid_entry(&path, "expected a regular package file"));
            }
        }
        copy_package_files(&package, &Path::new("plugins").join(self.name), ardent_dir)
    }
}

fn copy_package_files(
    source: &Path,
    target: &Path,
    ardent_dir: &Path,
) -> Result<(), (PathBuf, io::Error)> {
    let entries = fs::read_dir(source).map_err(|err| (source.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| (source.to_path_buf(), err))?;
        let path = entry.path();
        let target = target.join(entry.file_name());
        let kind = entry.file_type().map_err(|err| (path.clone(), err))?;
        if kind.is_dir() {
            copy_package_files(&path, &target, ardent_dir)?;
        } else if kind.is_file() {
            let bytes = fs::read(&path).map_err(|err| (path, err))?;
            layout::write_atomic(ardent_dir, &target, &bytes)
                .map_err(|err| (ardent_dir.join(target), err))?;
        } else {
            return Err(invalid_entry(&path, "expected a regular file or directory"));
        }
    }
    Ok(())
}

fn invalid_entry(path: &Path, message: &'static str) -> (PathBuf, io::Error) {
    (
        path.to_path_buf(),
        io::Error::new(io::ErrorKind::InvalidData, message),
    )
}
