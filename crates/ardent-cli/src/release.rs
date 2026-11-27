use std::collections::BTreeMap;
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
        project_ardent: &Path,
        destination_root: &Path,
    ) -> PackageResult<()> {
        let package = source.join(self.name);
        let files = entries(&package, Some(project_ardent), self.required_files)?;
        let target = Path::new("plugins").join(self.name);
        layout::create_dir(destination_root, &format!("plugins/{}", self.name))
            .map_err(|err| (destination_root.join(&target), err))?;
        for (relative, kind) in files {
            let destination = target.join(&relative);
            if kind.is_dir() {
                let dir = destination_root.join(destination);
                fs::create_dir(&dir).map_err(|err| (dir, err))?;
            } else {
                let path = package.join(relative);
                let bytes = fs::read(&path).map_err(|err| (path, err))?;
                layout::write_atomic(destination_root, &destination, &bytes)
                    .map_err(|err| (destination_root.join(destination), err))?;
            }
        }
        Ok(())
    }

    pub(super) fn differs(&self, release: &Path, project_ardent: &Path) -> PackageResult<bool> {
        let source = release.join(self.name);
        let installed = project_ardent.join("plugins").join(self.name);
        for dir in [release, &source, &installed] {
            if !fs::symlink_metadata(dir)
                .map_err(|err| (dir.to_owned(), err))?
                .is_dir()
            {
                return Err(invalid_entry(dir, "expected a package directory"));
            }
        }
        let bundled = entries(&source, Some(project_ardent), self.required_files)?;
        if entries(&installed, None, &[])? != bundled {
            return Ok(true);
        }
        for (path, kind) in bundled {
            if kind.is_file() && digest(&source.join(&path))? != digest(&installed.join(path))? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

type PackageResult<T> = Result<T, (PathBuf, io::Error)>;

fn digest(path: &Path) -> PackageResult<blake3::Hash> {
    let mut hash = blake3::Hasher::new();
    let file = fs::File::open(path).map_err(|err| (path.to_owned(), err))?;
    hash.update_reader(file)
        .map_err(|err| (path.to_owned(), err))?;
    Ok(hash.finalize())
}

fn entries(
    root: &Path,
    skip: Option<&Path>,
    required: &[&str],
) -> PackageResult<BTreeMap<PathBuf, fs::FileType>> {
    let mut files = BTreeMap::new();
    let mut dirs = vec![PathBuf::new()];
    while let Some(relative) = dirs.pop() {
        let dir = root.join(&relative);
        for entry in fs::read_dir(&dir).map_err(|err| (dir.clone(), err))? {
            let entry = entry.map_err(|err| (dir.clone(), err))?;
            let path = relative.join(entry.file_name());
            let kind = entry.file_type().map_err(|err| (entry.path(), err))?;
            if kind.is_dir() {
                if let Some(skip) = skip
                    && same_file::is_same_file(entry.path(), skip)
                        .map_err(|err| (entry.path(), err))?
                {
                    continue;
                }
                dirs.push(path.clone());
            } else if !kind.is_file() {
                return Err(invalid_entry(
                    &entry.path(),
                    "expected a regular file or directory",
                ));
            }
            files.insert(path, kind);
        }
    }
    for name in required {
        if !files
            .get(Path::new(name))
            .is_some_and(fs::FileType::is_file)
        {
            return Err(invalid_entry(
                &root.join(name),
                "missing regular package file with this exact name",
            ));
        }
    }
    Ok(files)
}

fn invalid_entry(path: &Path, message: &'static str) -> (PathBuf, io::Error) {
    (
        path.to_path_buf(),
        io::Error::new(io::ErrorKind::InvalidData, message),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_the_whole_tree_even_when_versions_match() -> io::Result<()> {
        let release = tempfile::tempdir()?;
        let project = tempfile::tempdir()?;
        let plugin = &RELEASE[0];
        let source = release.path().join(plugin.name);
        fs::create_dir_all(source.join("docs/empty"))?;
        for name in ["plugin.yaml", "format.wasm", "docs/readme"] {
            fs::write(source.join(name), name)?;
        }
        plugin
            .install(release.path(), project.path(), project.path())
            .map_err(|(_, err)| err)?;
        let installed = project.path().join("plugins").join(plugin.name);
        let differs = || {
            plugin
                .differs(release.path(), project.path())
                .map_err(|(_, err)| err)
        };
        assert!(!differs()?);
        for name in ["format.wasm", "docs/readme"] {
            fs::write(installed.join(name), "changed")?;
            assert!(differs()?);
            fs::write(installed.join(name), name)?;
        }
        fs::remove_dir(installed.join("docs/empty"))?;
        assert!(differs()?);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&source, installed.join("linked"))?;
            assert!(differs().is_err());
        }
        Ok(())
    }
}
