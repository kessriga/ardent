//! Helpers for the integration tests: temporary directories outside any project, and snapshots of directory trees.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// Every file and directory under a root, with the content of each file. A directory has no content.
pub type Tree = BTreeMap<PathBuf, Option<Vec<u8>>>;

/// Creates an empty directory for one test. Fails when a directory above it is already an Ardent project, because the
/// commands would then find that project.
pub fn temp_dir() -> io::Result<TempDir> {
    let dir = tempfile::tempdir()?;
    if let Some(root) = dir.path().ancestors().find(|d| d.join(".ardent").is_dir()) {
        return Err(io::Error::other(format!(
            "{} is an Ardent project; remove its .ardent/ or set TMPDIR to a directory outside any project",
            root.display()
        )));
    }
    Ok(dir)
}

pub struct Release {
    dir: TempDir,
    executable: PathBuf,
}

impl Release {
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

/// Stages the release on the Cargo binary's filesystem so it works with a noexec temporary directory.
pub fn stage_release() -> io::Result<Release> {
    let binary = Path::new(env!("CARGO_BIN_EXE_ardent"));
    let parent = binary
        .parent()
        .ok_or_else(|| io::Error::other("Cargo binary has no parent directory"))?;
    let dir = tempfile::tempdir_in(parent)?;
    let executable = dir
        .path()
        .join(format!("ardent{}", std::env::consts::EXE_SUFFIX));
    let release = Release { dir, executable };
    if fs::hard_link(binary, release.executable()).is_err() {
        fs::copy(binary, release.executable())?;
    }
    let plugins = release.path().join("plugins");
    fs::create_dir(&plugins)?;
    for (path, contents) in snapshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins"))?
    {
        match contents {
            Some(bytes) => fs::write(plugins.join(path), bytes)?,
            None => fs::create_dir(plugins.join(path))?,
        }
    }
    Ok(release)
}

/// Isolate child processes from the developer's Ardent settings, packages and compilation cache.
pub fn ardent_command(binary: impl AsRef<Path>) -> Command {
    let home = Path::new(env!("CARGO_BIN_EXE_ardent")).with_file_name("ardent-test-home");
    let mut command = Command::new(binary.as_ref());
    command
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_CACHE_HOME", home.join("cache"));
    command
}

pub fn run_init(project: &Path, args: &[&str]) -> io::Result<Output> {
    let release = stage_release()?;
    ardent_command(release.executable())
        .arg("init")
        .args(args)
        .current_dir(project)
        .output()
}

pub fn snapshot(root: &Path) -> io::Result<Tree> {
    let mut tree = Tree::new();
    collect(root, root, &mut tree)?;
    Ok(tree)
}

fn collect(root: &Path, dir: &Path, tree: &mut Tree) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let relative = path.strip_prefix(root).map_err(io::Error::other)?;
        if path.is_dir() {
            tree.insert(relative.to_path_buf(), None);
            collect(root, &path, tree)?;
        } else {
            tree.insert(relative.to_path_buf(), Some(fs::read(&path)?));
        }
    }
    Ok(())
}
