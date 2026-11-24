use std::env;
use std::ffi::OsStr;
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
        copy_package_files(
            &package,
            &Path::new("plugins").join(self.name),
            ardent_dir,
            self.required_files,
        )
    }
}

fn copy_package_files(
    source: &Path,
    target: &Path,
    ardent_dir: &Path,
    required: &[&str],
) -> Result<(), (PathBuf, io::Error)> {
    let mut missing = required.to_vec();
    let entries = fs::read_dir(source).map_err(|err| (source.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| (source.to_path_buf(), err))?;
        let path = entry.path();
        let name = entry.file_name();
        let target = target.join(&name);
        let kind = entry.file_type().map_err(|err| (path.clone(), err))?;
        if kind.is_dir() {
            if !same_file::is_same_file(&path, ardent_dir).map_err(|err| (path.clone(), err))? {
                copy_package_files(&path, &target, ardent_dir, &[])?;
            }
        } else if kind.is_file() {
            missing.retain(|file| OsStr::new(file) != name);
            let bytes = fs::read(&path).map_err(|err| (path, err))?;
            layout::write_atomic(ardent_dir, &target, &bytes)
                .map_err(|err| (ardent_dir.join(target), err))?;
        } else {
            return Err(invalid_entry(&path, "expected a regular file or directory"));
        }
    }
    if let Some(name) = missing.first() {
        return Err(invalid_entry(
            &source.join(name),
            "missing regular package file with this exact name",
        ));
    }
    Ok(())
}

fn invalid_entry(path: &Path, message: &'static str) -> (PathBuf, io::Error) {
    (
        path.to_path_buf(),
        io::Error::new(io::ErrorKind::InvalidData, message),
    )
}
