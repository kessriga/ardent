//! Git inputs outside the recursive project watch can disappear or be replaced atomically.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) struct Inputs {
    previous: Vec<(PathBuf, Option<Vec<u8>>)>,
    checked: Instant,
}

impl Inputs {
    pub(super) fn new(root: &Path) -> Self {
        Self {
            previous: read(root),
            checked: Instant::now(),
        }
    }

    pub(super) fn changed(&mut self, root: &Path) -> bool {
        if self.checked.elapsed() < Duration::from_secs(1) {
            return false;
        }
        self.checked = Instant::now();
        let next = read(root);
        if self.previous == next {
            return false;
        }
        self.previous = next;
        true
    }
}

fn read(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut inputs = Vec::new();
    if let Some(repo) = crate::tracked::git_root(root) {
        for ancestor in root.ancestors().skip(1) {
            if !ancestor.starts_with(repo) {
                break;
            }
            inputs.push(ancestor.join(".gitignore"));
        }
        let git = repo.join(".git");
        if git.is_dir() {
            inputs.push(git.join("info/exclude"));
        } else if let Some(common) = common_directory(repo, &git) {
            inputs.push(common.join("info/exclude"));
        }
        // A repository boundary can change which ignore files apply even if they keep their timestamps.
        inputs.push(git);
    }
    inputs
        .into_iter()
        .map(|path| {
            let bytes = path.is_file().then(|| fs::read(&path).ok()).flatten();
            (path, bytes)
        })
        .collect()
}

fn common_directory(repo: &Path, git_file: &Path) -> Option<PathBuf> {
    let pointer = fs::read_to_string(git_file).ok()?;
    let git = repo.join(pointer.lines().next()?.strip_prefix("gitdir: ")?);
    let common = fs::read_to_string(git.join("commondir")).ok()?;
    Some(git.join(common.lines().next()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_shared_excludes_of_a_linked_worktree() -> std::io::Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("checkout");
        let git = temp.path().join("common/worktrees/checkout");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(&git)?;
        fs::create_dir_all(temp.path().join("common/info"))?;
        fs::write(repo.join(".git"), format!("gitdir: {}\n", git.display()))?;
        fs::write(git.join("commondir"), "../..\n")?;
        let before = read(&repo);
        fs::write(temp.path().join("common/info/exclude"), "private.md")?;
        assert_ne!(before, read(&repo));
        Ok(())
    }

    #[test]
    fn detects_external_rule_creation_and_removal() -> std::io::Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let root = repo.join("work/project");
        fs::create_dir_all(repo.join(".git/info"))?;
        fs::create_dir_all(&root)?;
        let mut rules = Inputs::new(&root);
        let mut changed = || {
            rules.checked = Instant::now() - Duration::from_secs(2);
            assert!(rules.changed(&root));
        };
        fs::write(repo.join("work/.gitignore"), "README.md")?;
        changed();
        fs::remove_file(repo.join("work/.gitignore"))?;
        changed();
        fs::write(repo.join(".git/info/exclude"), "*.md")?;
        changed();
        let snapshot = read(&root);
        fs::write(temp.path().join(".gitignore"), "*.rs")?;
        assert_eq!(snapshot, read(&root));
        Ok(())
    }
}
