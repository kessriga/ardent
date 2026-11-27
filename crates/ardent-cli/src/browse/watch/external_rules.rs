//! Discovery inputs outside the recursive project watch can disappear or be replaced atomically.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ardent_core::layout::{ARDENT_DIR, CACHE_DIR};
use notify::{Config, Event, Watcher};
use same_file::Handle;

pub(super) struct Inputs {
    pub(super) linked_worktrees: Vec<PathBuf>,
    user_dir: Option<PathBuf>,
    previous: Snapshot,
    package_watch: Option<(Handle, Box<dyn Watcher>)>,
    package_event: Arc<AtomicBool>,
    checked: Instant,
}

#[derive(Debug, PartialEq)]
struct Snapshot {
    files: Vec<(PathBuf, Option<Vec<u8>>)>,
    packages: Option<BTreeSet<OsString>>,
}

impl Inputs {
    pub(super) fn new(root: &Path, user_dir: Option<PathBuf>) -> Self {
        let mut inputs = Self {
            linked_worktrees: Vec::new(),
            previous: read(root, &[], user_dir.as_deref()),
            user_dir,
            package_watch: None,
            package_event: Arc::default(),
            checked: Instant::now(),
        };
        inputs.watch_packages(root);
        inputs
    }

    pub(super) fn changed(&mut self, root: &Path) -> bool {
        if self.checked.elapsed() < Duration::from_secs(1) {
            return false;
        }
        self.checked = Instant::now();
        let watch_changed = self.watch_packages(root);
        let package_changed = self.package_event.swap(false, Ordering::Relaxed);
        let next = read(root, &self.linked_worktrees, self.user_dir.as_deref());
        if self.previous == next {
            return watch_changed || package_changed;
        }
        self.previous = next;
        true
    }

    // Reattach after the package tree is created or replaced, including through a managed config symlink.
    fn watch_packages(&mut self, root: &Path) -> bool {
        let Some((path, handle)) = self.user_dir.as_deref().and_then(package_directory) else {
            return self.package_watch.take().is_some();
        };
        if self
            .package_watch
            .as_ref()
            .is_some_and(|(watched, _)| *watched == handle)
        {
            return false;
        }
        let event = Arc::clone(&self.package_event);
        let cache = root.join(ARDENT_DIR).join(CACHE_DIR);
        let observe = move |result: notify::Result<Event>| {
            if let Ok(change) = result
                && (change.need_rescan()
                    || (!change.kind.is_access()
                        && change.paths.iter().any(|path| !path.starts_with(&cache))))
            {
                event.store(true, Ordering::Relaxed);
            }
        };
        let config = Config::default()
            .with_follow_symlinks(false)
            .with_poll_interval(Duration::from_secs(1))
            .with_compare_contents(true);
        let was_watched = self.package_watch.is_some();
        self.package_watch = super::watch_tree(&path, observe, config)
            .ok()
            .map(|(watcher, _)| (handle, watcher));
        was_watched || self.package_watch.is_some()
    }
}

fn package_directory(user: &Path) -> Option<(PathBuf, Handle)> {
    let path = user.canonicalize().ok()?.join("plugins");
    if !path.symlink_metadata().ok()?.is_dir() {
        return None;
    }
    let handle = Handle::from_path(&path).ok()?;
    Some((path, handle))
}

fn read(root: &Path, linked_worktrees: &[PathBuf], user_dir: Option<&Path>) -> Snapshot {
    let mut inputs: Vec<_> = user_dir
        .into_iter()
        .map(|dir| dir.join(ardent_core::config::FILE_NAME))
        .collect();
    let nearest = crate::tracked::git_root(root);
    if let Some(repo) = nearest {
        for ancestor in root.ancestors().skip(1) {
            if !ancestor.starts_with(repo) {
                break;
            }
            inputs.push(ancestor.join(".gitignore"));
        }
    }
    for repo in nearest
        .into_iter()
        .chain(linked_worktrees.iter().map(PathBuf::as_path))
    {
        let git = repo.join(".git");
        if git.is_dir() {
            inputs.push(git.join("info/exclude"));
        } else if let Some(common) = common_directory(repo, &git) {
            inputs.push(common.join("info/exclude"));
        }
        // A repository boundary can change which ignore files apply even if they keep their timestamps.
        inputs.push(git);
    }
    inputs.sort_unstable();
    inputs.dedup();
    let files = inputs
        .into_iter()
        .map(|path| {
            let bytes = path.is_file().then(|| fs::read(&path).ok()).flatten();
            (path, bytes)
        })
        .collect();
    let packages = user_dir.and_then(|dir| {
        fs::read_dir(dir.join("plugins"))
            .ok()?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<std::io::Result<_>>()
            .ok()
    });
    Snapshot { files, packages }
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
        let before = read(&repo, &[], None);
        fs::write(temp.path().join("common/info/exclude"), "private.md")?;
        assert_ne!(before, read(&repo, &[], None));
        Ok(())
    }

    #[test]
    fn detects_external_excludes_for_a_nested_worktree_even_when_its_sources_are_ignored()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::tracked::{self, Discovery, Patterns};
        use ardent_format::patterns;

        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        let nested = root.join("linked");
        let common = temp.path().join("external/.git");
        let git = common.join("worktrees/linked");
        fs::create_dir_all(root.join(".git"))?;
        fs::create_dir_all(&nested)?;
        fs::create_dir_all(&git)?;
        fs::create_dir(common.join("info"))?;
        fs::write(nested.join(".git"), format!("gitdir: {}\n", git.display()))?;
        fs::write(git.join("commondir"), "../..\n")?;
        fs::write(nested.join("example.md"), "# Nested")?;
        let exclude = common.join("info/exclude");
        fs::write(&exclude, "example.md\n")?;

        let mut patterns = Patterns::new(&root, &[])?;
        patterns.add("markdown", patterns::compile(&root, &["*.md".into()])?);
        let walk = tracked::walk(&root, &patterns);
        assert!(walk.files.is_empty());
        let discovery = Discovery::from_walk(&walk, &patterns);
        assert!(discovery.affects_files(&root, Path::new("linked")));
        let mut inputs = Inputs::new(&root, None);
        inputs.linked_worktrees = discovery.linked_worktrees;
        let mut changed = || {
            inputs.checked = Instant::now() - Duration::from_secs(2);
            inputs.changed(&root)
        };
        assert!(changed(), "recheck inputs discovered during the sync");
        assert!(!changed());
        fs::remove_file(&exclude)?;
        assert!(changed());
        assert_eq!(
            tracked::walk(&root, &patterns).files[0].path,
            "linked/example.md"
        );
        Ok(())
    }

    #[test]
    fn detects_external_rule_creation_and_removal() -> std::io::Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let root = repo.join("work/project");
        fs::create_dir_all(repo.join(".git/info"))?;
        fs::create_dir_all(&root)?;
        let mut rules = Inputs::new(&root, None);
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
        let snapshot = read(&root, &[], None);
        fs::write(temp.path().join(".gitignore"), "*.rs")?;
        assert_eq!(snapshot, read(&root, &[], None));
        Ok(())
    }
}
