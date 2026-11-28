//! Poll only the selected user palette. Keep the last usable colors during an incomplete save.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use ardent_core::theme::Theme;
use ardent_tui::BrowserTheme;

use crate::sync::source;

pub(super) struct ThemeWatcher {
    selection: Sender<Option<String>>,
    updates: Receiver<BrowserTheme>,
}

impl ThemeWatcher {
    pub fn start(name: Option<String>) -> io::Result<(Self, BrowserTheme)> {
        let user_dir = crate::xdg::config_dir();
        let mut active = ActiveTheme::default();
        active.refresh(user_dir.as_deref(), name.as_deref());
        let initial = active.palette.clone();
        let (selection, selected) = mpsc::channel();
        let (sender, updates) = mpsc::sync_channel(1);
        thread::Builder::new().spawn(move || {
            let mut name = name;
            loop {
                match selected.recv_timeout(Duration::from_secs(1)) {
                    Ok(new) => name = selected.try_iter().last().unwrap_or(new),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                if active.refresh(user_dir.as_deref(), name.as_deref())
                    && sender.send(active.palette.clone()).is_err()
                {
                    return;
                }
            }
        })?;
        Ok((Self { selection, updates }, initial))
    }

    pub fn select(&self, name: Option<String>) -> io::Result<()> {
        self.selection.send(name).map_err(io::Error::other)
    }

    pub fn ready(&self) -> io::Result<Option<BrowserTheme>> {
        match self.updates.try_recv() {
            Ok(palette) => Ok(Some(palette)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

#[derive(Default)]
struct ActiveTheme {
    input: Option<Result<(PathBuf, String), String>>,
    palette: BrowserTheme,
}

impl ActiveTheme {
    /// Compare the actual bytes read, so atomic saves and edits with identical timestamps are visible.
    fn refresh(&mut self, user_dir: Option<&Path>, name: Option<&str>) -> bool {
        let input = name.map(|name| read(user_dir, name));
        if self.input == input {
            return false;
        }
        self.input = input;
        let parsed = match &self.input {
            None => Ok(Theme::default()),
            Some(Err(message)) => Err(message.clone()),
            Some(Ok((path, text))) => {
                Theme::from_yaml(text).map_err(|err| format!("{}: {err}", path.display()))
            }
        };
        self.palette.diagnostic = match parsed {
            Ok(colors) => {
                self.palette.colors = colors;
                None
            }
            Err(message) => Some(message),
        };
        true
    }
}

fn read(user_dir: Option<&Path>, name: &str) -> Result<(PathBuf, String), String> {
    if matches!(name, "" | "." | "..")
        || name.contains(['/', '\\', '\0'])
        || (cfg!(windows) && name.contains(':'))
    {
        return Err(format!(
            "invalid theme name {name:?}: expected a filename stem"
        ));
    }
    let dir = user_dir.ok_or_else(|| {
        format!("cannot locate the user configuration directory for theme {name:?}")
    })?;
    let path = dir.join("themes").join(format!("{name}.yaml"));
    let text = source::open_regular(&path)
        .and_then(io::read_to_string)
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok((path, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_before_lookup() {
        for name in ["", ".", "..", "/tmp/a", "../a", "a/b", "a\\b"] {
            assert!(
                read(None, name)
                    .expect_err("invalid stem")
                    .contains("invalid theme name")
            );
        }
        #[cfg(windows)]
        assert!(
            read(None, "C:relative")
                .expect_err("drive prefix")
                .contains("invalid theme name")
        );
    }

    #[cfg(unix)]
    #[test]
    fn loads_a_theme_with_a_colon_in_its_name() -> io::Result<()> {
        let user = tempfile::tempdir()?;
        let themes = user.path().join("themes");
        std::fs::create_dir(&themes)?;
        let path = themes.join("evening:blue.yaml");
        std::fs::write(&path, "schema: 1")?;
        assert_eq!(
            read(Some(user.path()), "evening:blue").map_err(io::Error::other)?,
            (path, "schema: 1".into())
        );
        Ok(())
    }
}
