//! Run sync and the filesystem checks on a worker so a slow project cannot hold up terminal input.

use std::fs::{File, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use ardent_tui::FileList;

use super::watch::Changes;
use crate::sync::{self, Run};

/// A stale preview or a failed source read warrants another check under the project lock.
pub(super) enum Preview {
    Recheck(String),
    Open { path: String, revision: Revision },
}

impl Preview {
    fn needs_check(&self, root: &Path, poll: bool) -> Option<&str> {
        match self {
            Self::Recheck(path) => Some(path),
            Self::Open { path, revision } if poll && revision.changed(&root.join(path)) => {
                Some(path)
            }
            Self::Open { .. } => None,
        }
    }
}

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
    fn changed(&self, path: &Path) -> bool {
        !path
            .metadata()
            .is_ok_and(|meta| Self::identity(&meta) == self.identity)
    }

    #[cfg(not(unix))]
    pub(super) fn from_source(_metadata: &Metadata, hash: String) -> Self {
        Self { hash }
    }

    #[cfg(not(unix))]
    fn changed(&self, path: &Path) -> bool {
        use ardent_core::structure;
        use ardent_format::MAX_SOURCE_BYTES;
        // The fallback watcher cannot distinguish two revisions with the same size and mtime.
        let bytes = File::open(path).and_then(|file| {
            let size = file.metadata()?.len();
            sync::source::read(file, size, MAX_SOURCE_BYTES)
        });
        !bytes.is_ok_and(|bytes| structure::content_hash(&bytes) == self.hash)
    }
}

pub(super) type Update = (FileList, bool);

pub(super) struct Refresh {
    updates: Receiver<io::Result<Update>>,
    previews: mpsc::Sender<Preview>,
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
        Ok(Self { updates, previews })
    }

    pub(super) fn initial(&self) -> io::Result<Update> {
        self.updates.recv().map_err(io::Error::other)?
    }

    pub(super) fn opened(&self, preview: Preview) {
        // The update channel delivers any worker error to the event loop.
        let _ = self.previews.send(preview);
    }

    pub(super) fn ready(&self) -> io::Result<Option<Update>> {
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
    opened: Receiver<Preview>,
) -> io::Result<()> {
    // Register before the first sync to catch edits that happen during the scan.
    let mut changes = Changes::new(root)?;
    let mut denied: Vec<String> = Vec::new();
    let mut preview: Option<Preview> = None;
    let mut warnings = Some(Vec::new());
    let mut checked = Instant::now();
    loop {
        let check_access = checked.elapsed() >= Duration::from_secs(1);
        if check_access {
            checked = Instant::now();
        }
        let verify = preview
            .as_ref()
            .and_then(|p| p.needs_check(root, check_access && changes.is_polling()));
        // A permission repair can leave both size and modification time unchanged.
        let repaired = check_access
            && denied.iter().any(|path| {
                let source = root.join(path);
                source.symlink_metadata().is_ok_and(|m| m.is_file()) && File::open(source).is_ok()
            });
        if warnings.is_some() || verify.is_some() || repaired {
            let report = sync::run_at(root, Run::Browse { verify })
                .map_err(|err| io::Error::other(err.to_string()))?;
            if verify.is_some() {
                preview = None;
            }
            denied = report.read_denied;
            changes.update(report.discovery)?;
            let failed =
                !report.errors.is_empty() || report.files.iter().any(|(_, error)| error.is_some());
            let mut messages = report.errors;
            messages.extend(report.warnings);
            messages.extend(warnings.take().unwrap_or_default());
            let list = FileList {
                files: report.files,
                messages,
            };
            if updates.send(Ok((list, failed))).is_err() {
                return Ok(());
            }
        }
        match opened.recv_timeout(Duration::from_millis(75)) {
            Ok(latest) => preview = Some(latest),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        }
        warnings = changes.ready()?;
    }
}
