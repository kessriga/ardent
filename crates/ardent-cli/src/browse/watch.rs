use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ardent_core::layout::{ARDENT_DIR, CACHE_DIR};
use notify::{
    Config, Event, EventHandler, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher,
};

/// Collect filesystem notifications for the next browser sync. The watcher stays alive for the session.
pub(super) struct Changes {
    _watcher: Box<dyn Watcher>,
    pending: Arc<Mutex<Pending>>,
}

/// Coalesce notifications at the producer. A refresh waiting on the project lock can take arbitrarily long.
#[derive(Default)]
struct Pending {
    since: Option<Instant>,
    warnings: BTreeSet<String>,
    reported: BTreeSet<String>,
    fatal: Option<notify::Error>,
}

impl Pending {
    fn record(&mut self, event: notify::Result<Event>, cache: &Path) {
        match event {
            Ok(event) if relevant(&event, cache) => self.reported.clear(),
            Ok(_) => return,
            Err(error) if !error.paths.is_empty() => {
                if error.paths.iter().all(|path| path.starts_with(cache)) {
                    return;
                }
                let warning = format!("Watcher reported {error}");
                // Polling can report the same unreadable directory on every pass.
                if self.reported.contains(&warning) {
                    return;
                }
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
        let cache = root.join(ARDENT_DIR).join(CACHE_DIR);
        let pending = Arc::new(Mutex::new(Pending::default()));
        let batch = Arc::clone(&pending);
        let handle = move |event| {
            if let Ok(mut pending) = batch.lock() {
                pending.record(event, &cache);
            }
        };
        let config = Config::default()
            .with_follow_symlinks(false)
            .with_poll_interval(Duration::from_secs(1));
        let watcher = start::<RecommendedWatcher>(&root, handle.clone(), config.clone())
            .or_else(|_| start::<PollWatcher>(&root, handle, config))
            .map_err(io::Error::other)?;
        Ok(Self {
            _watcher: watcher,
            pending,
        })
    }

    /// Return new path diagnostics when a batch of changes needs a sync.
    pub(super) fn ready(&self) -> io::Result<Option<Vec<String>>> {
        self.pending
            .lock()
            .map_err(|_| io::Error::other("watcher state poisoned"))?
            .take_ready()
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

/// Sync writes its own cache. Ignore those notifications, but honor a request to rescan after lost events.
fn relevant(event: &Event, cache: &Path) -> bool {
    !matches!(event.kind, EventKind::Access(_))
        && (event.need_rescan() || event.paths.iter().any(|path| !path.starts_with(cache)))
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
    fn cache_writes_do_not_trigger_another_sync() {
        let cache = Path::new("/project/.ardent/cache");
        let event =
            Event::new(EventKind::Any).add_path(cache.join("files/README.md/structure.yaml"));
        assert!(!relevant(&event, cache));
        assert!(relevant(
            &event.add_path("/project/README.md".into()),
            cache
        ));
    }
}
