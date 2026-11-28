//! Run sync and the filesystem checks on a worker so a slow project cannot hold up terminal input.

use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use ardent_format::FormatPlugin;
use ardent_tui::FileList;

use super::theme::ActiveTheme;
use super::watch::{Batch, Changes};
use crate::sync::{self, Run, source};
use crate::update;

/// The source currently on screen, including a file whose scan failed.
/// Record its revision before verifying it, so a second edit during sync is still detectable.
pub(super) struct Preview {
    path: String,
    revision: Option<Revision>,
    recheck: bool,
}

impl Preview {
    pub(super) fn loaded(path: &str, revision: Revision) -> Self {
        Self {
            path: path.to_owned(),
            revision: Some(revision),
            recheck: false,
        }
    }

    pub(super) fn recheck(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            revision: None,
            recheck: true,
        }
    }

    fn needs_check(&mut self, root: &Path, poll: bool) -> Option<&str> {
        if !self.recheck && !poll {
            return None;
        }
        let current = Revision::read(&root.join(&self.path));
        if !self.recheck && current == self.revision {
            return None;
        }
        self.revision = current;
        self.recheck = false;
        Some(&self.path)
    }
}

#[derive(PartialEq, Eq)]
pub(super) struct Revision {
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
    #[cfg(not(unix))]
    hash: String,
}

impl Revision {
    #[cfg(unix)]
    pub(super) fn from_source(metadata: &Metadata, _hash: String) -> Self {
        Self {
            identity: Self::identity(metadata),
        }
    }

    #[cfg(unix)]
    fn identity(metadata: &Metadata) -> (u64, u64, i64, i64) {
        use std::os::unix::fs::MetadataExt;
        // An in-place edit can restore mtime. A rename can also replace the inode.
        (
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        )
    }

    #[cfg(unix)]
    fn read(path: &Path) -> Option<Self> {
        let meta = path.symlink_metadata().ok()?;
        meta.is_file().then(|| Self {
            identity: Self::identity(&meta),
        })
    }

    #[cfg(not(unix))]
    pub(super) fn from_source(_metadata: &Metadata, hash: String) -> Self {
        Self { hash }
    }

    #[cfg(not(unix))]
    fn read(path: &Path) -> Option<Self> {
        use ardent_core::structure;
        use ardent_format::MAX_SOURCE_BYTES;
        // The fallback watcher cannot distinguish two revisions with the same size and mtime.
        let file = source::open_regular(path).ok()?;
        let size = file.metadata().ok()?.len();
        let bytes = source::read(file, size, MAX_SOURCE_BYTES).ok()?;
        Some(Self {
            hash: structure::content_hash(&bytes),
        })
    }
}

pub(super) struct Update {
    pub files: FileList,
    pub failed: bool,
    pub formats: Vec<FormatPlugin>,
}

pub(super) struct Refresh {
    updates: Receiver<io::Result<Update>>,
    previews: mpsc::Sender<Option<Preview>>,
    displayed: Option<String>,
}

impl Refresh {
    pub(super) fn start(root: PathBuf) -> io::Result<Self> {
        let (sender, updates) = mpsc::sync_channel(1);
        let (previews, opened) = mpsc::channel();
        thread::Builder::new().spawn(move || {
            if let Err(error) = watch(&root, &sender, opened) {
                let _ = sender.send(Err(error));
            }
        })?;
        Ok(Self {
            updates,
            previews,
            displayed: None,
        })
    }

    pub(super) fn initial(&self) -> io::Result<Update> {
        self.updates.recv().map_err(io::Error::other)?
    }

    pub(super) fn ready(
        &mut self,
        path: Option<&str>,
        observed: Option<Preview>,
    ) -> io::Result<Option<Update>> {
        // Successful loads supply their exact revision. Error screens can open without loading
        // a document, and returning to the picker ends monitoring of the former source.
        if self.displayed.as_deref() != path || observed.is_some() {
            let selected = path.map(|path| {
                observed
                    .filter(|p| p.path == path)
                    .unwrap_or_else(|| Preview::recheck(path))
            });
            self.displayed = path.map(str::to_owned);
            let _ = self.previews.send(selected);
        }
        match self.updates.try_recv() {
            Ok(update) => update.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

fn watch(
    root: &Path,
    updates: &SyncSender<io::Result<Update>>,
    opened: Receiver<Option<Preview>>,
) -> io::Result<()> {
    // Register before the first sync to catch edits that happen during the scan.
    let mut changes = Changes::new(root)?;
    let mut denied: Vec<String> = Vec::new();
    let mut preview: Option<Preview> = None;
    let mut batch = Some(Batch {
        warnings: Vec::new(),
        check_plugins: true,
    });
    let mut available = Ok(update::Report::default());
    let user_dir = crate::xdg::config_dir();
    let mut theme = ActiveTheme::default();
    let mut checked = Instant::now();
    loop {
        let check_access = checked.elapsed() >= Duration::from_secs(1);
        if check_access {
            checked = Instant::now();
        }
        let verify = preview
            .as_mut()
            .and_then(|p| p.needs_check(root, check_access && changes.is_polling()));
        // A permission repair can leave both size and modification time unchanged.
        let repaired = check_access
            && denied
                .iter()
                .any(|path| source::open_regular(&root.join(path)).is_ok());
        let theme_changed = check_access && theme.refresh(user_dir.as_deref());
        if batch.is_some() || verify.is_some() || repaired || theme_changed {
            let report = sync::run_at(root, Run::Browse { verify })
                .map_err(|err| io::Error::other(err.to_string()))?;
            if let Some(open) = &preview
                && report
                    .files
                    .binary_search_by(|(path, _)| path.cmp(&open.path))
                    .is_err()
            {
                preview = None;
            }
            denied = report.read_denied;
            changes.update(report.discovery)?;
            let failed =
                !report.errors.is_empty() || report.files.iter().any(|(_, error)| error.is_some());
            let mut messages = report.errors;
            messages.extend(report.warnings);
            if theme.name != report.theme {
                theme.name = report.theme;
                theme.refresh(user_dir.as_deref());
            }
            messages.extend(theme.diagnostic.iter().cloned());
            if let Some(batch) = batch.take() {
                messages.extend(batch.warnings);
                if batch.check_plugins {
                    available = update::available(root);
                }
            }
            let update_notice = match &available {
                Ok(report) => {
                    messages.extend(
                        report
                            .errors
                            .iter()
                            .map(|e| format!("Cannot check plugin updates: {e}")),
                    );
                    report
                        .names()
                        .map(|names| format!("Plugin updates: {names}. Run ardent update"))
                }
                Err(error) => {
                    messages.push(format!("Cannot check plugin updates: {error}"));
                    None
                }
            };
            let list = FileList {
                theme: theme.colors.clone(),
                files: report.files,
                messages,
                update_notice,
            };
            if updates
                .send(Ok(Update {
                    files: list,
                    failed,
                    formats: report.browse_formats,
                }))
                .is_err()
            {
                return Ok(());
            }
        }
        match opened.recv_timeout(Duration::from_millis(75)) {
            Ok(latest) => preview = opened.try_iter().last().unwrap_or(latest),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        batch = changes.ready()?;
    }
}
