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

/// Stages the Cargo binary and its release packages side by side, as distributed.
pub fn stage_release(dir: &Path) -> io::Result<PathBuf> {
    let binary = Path::new(env!("CARGO_BIN_EXE_ardent"));
    let staged = dir.join(format!("ardent{}", std::env::consts::EXE_SUFFIX));
    if fs::hard_link(binary, &staged).is_err() {
        fs::copy(binary, &staged)?;
    }
    let plugins = dir.join("plugins");
    fs::create_dir(&plugins)?;
    for (path, contents) in snapshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins"))?
    {
        match contents {
            Some(bytes) => fs::write(plugins.join(path), bytes)?,
            None => fs::create_dir(plugins.join(path))?,
        }
    }
    Ok(staged)
}

pub fn run_init(project: &Path, args: &[&str]) -> io::Result<Output> {
    let release = tempfile::tempdir()?;
    Command::new(stage_release(release.path())?)
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
