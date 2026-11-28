//! Poll only the selected user palette. Keep the last usable colors during an incomplete save.

use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::thread;
use std::time::Duration;

use ardent_core::theme::Theme;
use ardent_tui::BrowserTheme;

use crate::sync::source;

mod selector;
use selector::{Selection, Selectors};

const MAX_THEME_BYTES: u64 = 1 << 20;

pub(super) struct ThemeWatcher {
    recheck: SyncSender<()>,
    updates: Receiver<io::Result<Appearance>>,
}

pub(super) struct Appearance {
    pub palette: BrowserTheme,
    pub invalid_selector: bool,
}

impl ThemeWatcher {
    pub fn start(root: &Path) -> io::Result<(Self, BrowserTheme)> {
        let user_dir = crate::xdg::config_dir();
        let mut selectors = Selectors::new(root, user_dir.as_deref());
        let mut poll = move || {
            selectors
                .read()
                .map(|selection| Input::from_selection(user_dir.as_deref(), selection))
                .map_err(|err| io::Error::other(err.to_string()))
        };
        let first = poll()?;
        if let Input::InvalidSelector(error) = first {
            return Err(io::Error::other(error));
        }
        let mut active = ActiveTheme::default();
        let initial = active.apply(first).map(|a| a.palette).unwrap_or_default();
        let (recheck, requests) = mpsc::sync_channel(1);
        let (sender, updates) = mpsc::sync_channel(1);
        thread::Builder::new().spawn(move || {
            loop {
                match requests.recv_timeout(Duration::from_secs(1)) {
                    Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                let input = match poll() {
                    Ok(input) => input,
                    Err(err) => {
                        let _ = sender.send(Err(err));
                        return;
                    }
                };
                if let Some(appearance) = active.apply(input)
                    && sender.send(Ok(appearance)).is_err()
                {
                    return;
                }
            }
        })?;
        Ok((Self { recheck, updates }, initial))
    }

    pub fn recheck(&self) {
        let _ = self.recheck.try_send(());
    }

    pub fn ready(&self) -> io::Result<Option<Appearance>> {
        match self.updates.try_recv() {
            Ok(update) => update.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

#[derive(Default, PartialEq)]
enum Input {
    #[default]
    BuiltIn,
    File(PathBuf, String),
    InvalidPalette(String),
    InvalidSelector(String),
}

impl Input {
    fn from_selection(user_dir: Option<&Path>, selection: Selection) -> Self {
        match selection {
            Selection::Inherit => Self::BuiltIn,
            Selection::Invalid(error) => Self::InvalidSelector(error),
            Selection::Named(name) => match read(user_dir, &name) {
                Ok((path, text)) => Self::File(path, text),
                Err(error) => Self::InvalidPalette(error),
            },
        }
    }
}

#[derive(Default)]
struct ActiveTheme {
    input: Input,
    colors: Theme,
}

impl ActiveTheme {
    fn apply(&mut self, input: Input) -> Option<Appearance> {
        if input == self.input {
            return None;
        }
        let parsed = match &input {
            Input::BuiltIn => Ok(Theme::default()),
            Input::File(path, text) => {
                Theme::from_yaml(text).map_err(|err| format!("{}: {err}", path.display()))
            }
            Input::InvalidPalette(error) | Input::InvalidSelector(error) => Err(error.clone()),
        };
        self.input = input;
        let diagnostic = match parsed {
            Ok(colors) => {
                self.colors = colors;
                None
            }
            Err(message) => Some(message),
        };
        Some(Appearance {
            palette: BrowserTheme {
                colors: self.colors.clone(),
                diagnostic,
            },
            invalid_selector: matches!(self.input, Input::InvalidSelector(_)),
        })
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
        .and_then(|file| {
            let too_large =
                || io::Error::new(io::ErrorKind::InvalidData, "theme exceeds the 1 MiB limit");
            let size = file.metadata()?.len();
            if size > MAX_THEME_BYTES {
                return Err(too_large());
            }
            let mut bytes = Vec::with_capacity(size as usize);
            if file.take(MAX_THEME_BYTES + 1).read_to_end(&mut bytes)? as u64 > MAX_THEME_BYTES {
                return Err(too_large());
            }
            String::from_utf8(bytes).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
        })
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok((path, text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

    #[test]
    fn enforces_the_palette_size_limit_and_recovers() -> io::Result<()> {
        let home = tempfile::tempdir()?;
        let dir = home.path().join("themes");
        fs::create_dir(&dir)?;
        let path = dir.join("daylight.yaml");
        let mut text = "schema: 1\nsource: {heading: '#123456'}\n#".to_owned();
        text.extend(std::iter::repeat_n(
            ' ',
            MAX_THEME_BYTES as usize - text.len(),
        ));
        fs::write(&path, &text)?;
        let load = || Input::from_selection(Some(home.path()), Selection::Named("daylight".into()));
        let mut active = ActiveTheme::default();
        let first = active.apply(load()).expect("loaded palette");
        assert!(first.palette.diagnostic.is_none());
        assert_eq!(first.palette.colors.source.len(), 1);
        fs::OpenOptions::new()
            .write(true)
            .open(&path)?
            .set_len(MAX_THEME_BYTES + 1)?;
        let too_large = active.apply(load()).expect("size diagnostic");
        assert!(
            too_large
                .palette
                .diagnostic
                .is_some_and(|s| s.contains("daylight.yaml") && s.contains("1 MiB"))
        );
        assert_eq!(too_large.palette.colors.source, first.palette.colors.source);
        fs::write(&path, "schema: 1")?;
        let repaired = active.apply(load()).expect("repaired palette");
        assert!(repaired.palette.diagnostic.is_none());
        assert!(repaired.palette.colors.source.is_empty());
        Ok(())
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
