use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ardent_core::layout::{ARDENT_DIR, CACHE_DIR};
use notify::event::{MetadataKind, ModifyKind};
use notify::{
    Config, Event, EventHandler, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher,
};

use crate::tracked::Discovery;

mod external_rules;

/// Collect filesystem notifications for the next browser sync. The watcher stays alive for the session.
pub(super) struct Changes {
    _watcher: Box<dyn Watcher>,
    pending: Arc<Mutex<Pending>>,
    root: PathBuf,
    rules: external_rules::Inputs,
    polling: bool,
}

pub(super) struct Batch {
    pub warnings: Vec<String>,
    pub check_plugins: bool,
}

/// Coalesce notifications at the producer. A refresh waiting on the project lock can take arbitrarily long.
#[derive(Default)]
struct Pending {
    since: Option<Instant>,
    plugins_changed: bool,
    discovery: Option<Arc<Discovery>>,
    warnings: BTreeSet<String>,
    reported: BTreeSet<String>,
    problem_paths: BTreeSet<PathBuf>,
    fatal: Option<notify::Error>,
}

impl Pending {
    fn record(&mut self, event: notify::Result<Event>, cache: &Path) {
        match event {
            Ok(event) => {
                let plugins = cache.with_file_name("plugins");
                self.plugins_changed |= event.need_rescan()
                    || event
                        .paths
                        .iter()
                        .any(|path| path.starts_with(&plugins) || plugins.starts_with(path));
                self.reported.clear();
                for path in event.paths {
                    self.problem_paths.remove(&path);
                }
            }
            Err(error) if !error.paths.is_empty() => {
                if error.paths.iter().all(|path| path.starts_with(cache)) {
                    return;
                }
                let warning = format!("Watcher reported {error}");
                // Polling can report the same unreadable directory on every pass.
                if self.reported.contains(&warning) {
                    return;
                }
                self.problem_paths.extend(error.paths);
                self.warnings.insert(warning);
            }
            Err(error) => {
                self.fatal = Some(error);
                return;
            }
        }
        self.since.get_or_insert_with(Instant::now);
    }

    fn take_ready(&mut self) -> io::Result<Option<Vec<String>>> {
        if let Some(error) = self.fatal.take() {
            return Err(io::Error::other(error));
        }
        if self
            .since
            .is_some_and(|start| start.elapsed() >= Duration::from_millis(150))
        {
            self.since = None;
            let warnings = std::mem::take(&mut self.warnings);
            self.reported.extend(warnings.iter().cloned());
            return Ok(Some(warnings.into_iter().collect()));
        }
        Ok(None)
    }
}

impl Changes {
    pub(super) fn new(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        let pending = Arc::new(Mutex::new(Pending::default()));
        let project = root.clone();
        let batch = Arc::clone(&pending);
        let handle = move |event| observe(&batch, &project, event);
        let config = Config::default()
            .with_follow_symlinks(false)
            .with_poll_interval(Duration::from_secs(1));
        let (watcher, polling) =
            match start::<RecommendedWatcher>(&root, handle.clone(), config.clone()) {
                Ok(watcher) => (watcher, false),
                Err(_) => (
                    start::<PollWatcher>(&root, handle, config).map_err(io::Error::other)?,
                    true,
                ),
            };
        let rules = external_rules::Inputs::new(&root, crate::xdg::config_dir());
        Ok(Self {
            _watcher: watcher,
            pending,
            root,
            rules,
            polling,
        })
    }

    pub(super) fn is_polling(&self) -> bool {
        self.polling
    }

    pub(super) fn update(&mut self, mut discovery: Option<Discovery>) -> io::Result<()> {
        self.rules.linked_worktrees = discovery
            .as_mut()
            .map(|current| std::mem::take(&mut current.linked_worktrees))
            .unwrap_or_default();
        update_discovery(&self.pending, &self.root, discovery)
    }

    /// Return new path diagnostics when a batch of changes needs a sync.
    pub(super) fn ready(&mut self) -> io::Result<Option<Batch>> {
        let rules_changed = self.rules.changed(&self.root);
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| io::Error::other("watcher state poisoned"))?;
        if rules_changed {
            pending.discovery = None;
            pending.since.get_or_insert_with(Instant::now);
        }
        Ok(pending.take_ready()?.map(|warnings| Batch {
            warnings,
            check_plugins: std::mem::take(&mut pending.plugins_changed),
        }))
    }
}

fn start<W: Watcher + 'static>(
    root: &Path,
    handle: impl EventHandler,
    config: Config,
) -> notify::Result<Box<dyn Watcher>> {
    let mut watcher = W::new(handle, config)?;
    watcher.watch(root, RecursiveMode::Recursive)?;
    Ok(Box::new(watcher))
}

fn update_discovery(
    batch: &Mutex<Pending>,
    root: &Path,
    discovery: Option<Discovery>,
) -> io::Result<()> {
    let discovery = discovery.map(Arc::new);
    let lock = || {
        batch
            .lock()
            .map_err(|_| io::Error::other("watcher state poisoned"))
    };
    let previous = std::mem::replace(&mut lock()?.discovery, discovery.clone());
    // Later removals use the new membership. Check for removals the previous membership could have missed.
    if let (Some(current), Some(previous)) = (discovery, previous)
        && current.lost_new_paths(root, &previous)
    {
        lock()?.since.get_or_insert_with(Instant::now);
    }
    Ok(())
}

fn observe(batch: &Mutex<Pending>, root: &Path, event: notify::Result<Event>) {
    if let Ok(event) = &event {
        if event.kind.is_access() && !event.need_rescan() {
            return;
        }
        let (discovery, recovering) = {
            let Ok(mut pending) = batch.lock() else {
                return;
            };
            if event
                .paths
                .iter()
                .any(|path| path.strip_prefix(root).is_ok_and(project_rules))
            {
                // The next source event may already use the new format and exclude patterns.
                pending.discovery = None;
            }
            (
                pending.discovery.clone(),
                event
                    .paths
                    .iter()
                    .any(|p| pending.problem_paths.contains(p)),
            )
        };
        // A probe can read a moved directory. Release the batch lock before walking it.
        if !recovering && !relevant(event, root, discovery.as_deref()) {
            return;
        }
    }
    if let Ok(mut pending) = batch.lock() {
        pending.record(event, &root.join(ARDENT_DIR).join(CACHE_DIR));
    }
}

fn project_rules(relative: &Path) -> bool {
    relative == Path::new(ARDENT_DIR).join(ardent_core::config::FILE_NAME)
        || relative.starts_with(Path::new(ARDENT_DIR).join("plugins"))
}

/// Sync writes its own cache. Root and directory timestamp changes can also be side effects of ignored writes.
fn relevant(event: &Event, root: &Path, discovery: Option<&Discovery>) -> bool {
    if event.need_rescan() {
        return true;
    }
    event.paths.iter().any(|path| {
        let Ok(relative) = path.strip_prefix(root) else {
            return false;
        };
        if relative.starts_with(Path::new(ARDENT_DIR).join(CACHE_DIR)) {
            return false;
        }
        if path.is_dir()
            && matches!(
                event.kind,
                EventKind::Modify(
                    ModifyKind::Metadata(MetadataKind::WriteTime) | ModifyKind::Data(_)
                )
            )
        {
            return false;
        }
        if relative.as_os_str().is_empty() || relative == Path::new(ARDENT_DIR) {
            return true;
        }
        if relative.starts_with(ARDENT_DIR) {
            return project_rules(relative);
        }
        let Some(discovery) = discovery else {
            return true;
        };
        let parent = relative.parent().unwrap_or(Path::new(""));
        match relative.file_name().and_then(|name| name.to_str()) {
            Some(".gitignore" | ".git" | ARDENT_DIR) => {
                discovery.visible_directory(root, parent) || discovery.affects_files(root, parent)
            }
            Some("info") if relative.ends_with(".git/info") => discovery
                .visible_directory(root, relative.ancestors().nth(2).unwrap_or(Path::new(""))),
            Some("exclude") if relative.ends_with(".git/info/exclude") => discovery
                .visible_directory(root, relative.ancestors().nth(3).unwrap_or(Path::new(""))),
            _ => discovery.affects_files(root, relative),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_error_keeps_other_paths_watchable() -> io::Result<()> {
        let mut pending = Pending::default();
        let cache = Path::new("/project/.ardent/cache");
        let denied = || {
            notify::Error::io(io::ErrorKind::PermissionDenied.into())
                .add_path("/project/private".into())
        };
        pending.record(Err(denied()), cache);
        pending.since = Some(Instant::now() - Duration::from_secs(1));
        let warnings = pending
            .take_ready()?
            .expect("path warning requests a refresh");
        assert!(warnings[0].contains("/project/private"));
        pending.record(Err(denied()), cache);
        assert!(pending.since.is_none());
        pending.record(
            Ok(Event::new(EventKind::Any).add_path("/project/notes.md".into())),
            cache,
        );
        assert!(pending.since.is_some());
        pending.record(
            Ok(Event::new(EventKind::Any).add_path("/project/private".into())),
            cache,
        );
        pending.since = Some(Instant::now() - Duration::from_secs(1));
        assert_eq!(pending.take_ready()?, Some(Vec::new()));
        pending.record(Err(notify::Error::generic("watch stopped")), cache);
        assert!(pending.take_ready().is_err());
        Ok(())
    }

    #[test]
    fn filters_ignored_directories_but_discovers_moved_and_new_sources()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::tracked::{self, Patterns};
        use ardent_format::patterns;
        use std::fs;

        let temp = tempfile::tempdir()?;
        let root = temp.path();
        fs::write(root.join(".gitignore"), "target/\n*.generated.md\n")?;
        fs::create_dir(root.join("docs"))?;
        fs::write(root.join("docs/first.md"), "first")?;
        fs::create_dir_all(root.join("inner/.ardent"))?;
        fs::write(root.join("inner/inner.md"), "nested project")?;
        let mut patterns = Patterns::new(root, &["vendor/".into()])?;
        patterns.add("markdown", patterns::compile(root, &["*.md".into()])?);
        let walk = tracked::walk(root, &patterns);
        let snapshot = Discovery::from_walk(&walk, &patterns);
        let changed = |name: &str, kind| {
            relevant(
                &Event::new(kind).add_path(root.join(name)),
                root,
                Some(&snapshot),
            )
        };
        for dir in ["target/debug", "vendor/cache", "src/new"] {
            fs::create_dir_all(root.join(dir))?;
            fs::write(root.join(dir).join("output.md"), "new")?;
        }
        fs::write(root.join("ignored.generated.md"), "ignored")?;
        for name in [
            "target",
            "target/debug",
            "target/debug/output.md",
            "vendor/cache/output.md",
            "ignored.generated.md",
        ] {
            assert!(!changed(name, EventKind::Any), "{name}");
        }
        for name in [
            "src/new",
            "src/new/output.md",
            ".gitignore",
            ".ardent/config.yaml",
        ] {
            assert!(changed(name, EventKind::Any), "{name}");
        }
        assert!(!changed(
            "",
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime))
        ));
        fs::remove_dir(root.join("inner/.ardent"))?;
        assert!(changed("inner/.ardent", EventKind::Any));
        fs::remove_dir_all(root.join("docs"))?;
        assert!(changed("docs", EventKind::Any));
        Ok(())
    }

    #[test]
    fn removing_a_path_with_a_discovery_warning_refreshes_the_file_list()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::tracked::{self, Patterns};
        use ardent_format::patterns;
        use std::fs;

        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let mut patterns = Patterns::new(root, &[])?;
        for format in ["first", "second"] {
            patterns.add(format, patterns::compile(root, &["*.md".into()])?);
        }
        let discover = || Discovery::from_walk(&tracked::walk(root, &patterns), &patterns);
        let before = discover();
        fs::create_dir_all(root.join("docs"))?;
        fs::write(root.join("docs/conflict.md"), "# Ambiguous")?;
        fs::create_dir_all(root.join("tools/inner/.ardent"))?;
        let snapshot = discover();
        fs::remove_dir_all(root.join("docs"))?;
        fs::remove_dir_all(root.join("tools"))?;
        for path in ["docs/conflict.md", "docs", "tools/inner/.ardent", "tools"] {
            let removed = Event::new(EventKind::Remove(notify::event::RemoveKind::Any))
                .add_path(root.join(path));
            assert!(relevant(&removed, root, Some(&snapshot)), "{path}");
            assert!(!relevant(&removed, root, Some(&before)), "{path}");
        }
        let batch = Mutex::new(Pending::default());
        update_discovery(&batch, root, Some(before))?;
        update_discovery(&batch, root, Some(snapshot))?;
        assert!(batch.lock().expect("pending").since.is_some());
        Ok(())
    }

    #[test]
    fn reconciles_a_new_source_removed_while_sync_was_publishing()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::tracked::{self, Patterns};
        use ardent_format::patterns;
        use std::fs;
        let temp = tempfile::tempdir()?;
        let root = temp.path();
        let mut patterns = Patterns::new(root, &[])?;
        patterns.add("markdown", patterns::compile(root, &["*.md".into()])?);
        let discover = || {
            Some(Discovery::from_walk(
                &tracked::walk(root, &patterns),
                &patterns,
            ))
        };
        let pending = Mutex::new(Pending::default());
        update_discovery(&pending, root, discover())?;
        let source = root.join("new.md");
        fs::write(&source, "new")?;
        let scanned = discover();
        fs::remove_file(&source)?;
        observe(
            &pending,
            root,
            Ok(Event::new(EventKind::Remove(notify::event::RemoveKind::File)).add_path(source)),
        );
        assert!(pending.lock().expect("pending").since.is_none());
        update_discovery(&pending, root, scanned)?;
        assert!(pending.lock().expect("pending").since.is_some());
        Ok(())
    }

    #[test]
    fn cache_writes_do_not_trigger_another_sync() {
        let cache = Path::new("/project/.ardent/cache");
        let event =
            Event::new(EventKind::Any).add_path(cache.join("files/README.md/structure.yaml"));
        assert!(!relevant(&event, Path::new("/project"), None));
        assert!(relevant(
            &event.add_path("/project/README.md".into()),
            Path::new("/project"),
            None
        ));
    }
}
