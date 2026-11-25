use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use ardent_core::layout::{ARDENT_DIR, CACHE_DIR};
use notify::{
    Config, Event, EventHandler, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher,
};

pub(super) struct Changes {
    _watcher: Box<dyn Watcher>,
    events: Receiver<notify::Result<()>>,
    since: Option<Instant>,
}

impl Changes {
    pub(super) fn new(root: &Path) -> io::Result<Self> {
        let root = root.canonicalize()?;
        let cache = root.join(ARDENT_DIR).join(CACHE_DIR);
        let (sender, events) = mpsc::channel();
        let handle = move |event: notify::Result<Event>| {
            let result = match event {
                Ok(event) if relevant(&event, &cache) => Ok(()),
                Ok(_) => return,
                Err(error) => Err(error),
            };
            let _ = sender.send(result);
        };
        let config = Config::default()
            .with_follow_symlinks(false)
            .with_poll_interval(Duration::from_secs(1));
        let watcher = start::<RecommendedWatcher>(&root, handle.clone(), config.clone())
            .or_else(|_| start::<PollWatcher>(&root, handle, config))
            .map_err(io::Error::other)?;
        Ok(Self {
            _watcher: watcher,
            events,
            since: None,
        })
    }

    pub(super) fn ready(&mut self) -> io::Result<bool> {
        for event in self.events.try_iter() {
            event.map_err(io::Error::other)?;
            self.since.get_or_insert_with(Instant::now);
        }
        if self
            .since
            .is_some_and(|start| start.elapsed() >= Duration::from_millis(150))
        {
            self.since = None;
            return Ok(true);
        }
        Ok(false)
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

fn relevant(event: &Event, cache: &Path) -> bool {
    !matches!(event.kind, EventKind::Access(_))
        && (event.need_rescan() || event.paths.iter().any(|path| !path.starts_with(cache)))
}

#[cfg(test)]
mod tests {
    use super::*;

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
