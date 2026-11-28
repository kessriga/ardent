//! Resolve the project and user selectors without acquiring the project sync lock.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ardent_core::config::{self, Config, ConfigError};
use ardent_core::layout::ARDENT_DIR;

use crate::sync::SyncError;

#[derive(Clone, Default)]
pub(super) enum Selection {
    #[default]
    Inherit,
    Named(String),
    Invalid(String),
}

pub(super) struct Selectors {
    project: SelectorFile,
    user: Option<SelectorFile>,
}

impl Selectors {
    pub(super) fn new(root: &Path, user_dir: Option<&Path>) -> Self {
        Self {
            project: SelectorFile::new(&root.join(ARDENT_DIR)),
            user: user_dir.map(SelectorFile::new),
        }
    }

    pub(super) fn read(&mut self) -> Result<Selection, SyncError> {
        let project = self.project.read()?;
        let user = match self.user.as_mut().map(SelectorFile::read).transpose() {
            Ok(name) => name.unwrap_or_default(),
            Err(SyncError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                Selection::Inherit
            }
            Err(err) => return Err(err),
        };
        Ok(match (project, user) {
            (Selection::Inherit, user) | (_, user @ Selection::Invalid(_)) => user,
            (project, _) => project,
        })
    }
}

struct SelectorFile {
    path: PathBuf,
    text: Option<String>,
    selection: Selection,
}

impl SelectorFile {
    fn new(directory: &Path) -> Self {
        Self {
            path: directory.join(config::FILE_NAME),
            text: None,
            selection: Selection::Inherit,
        }
    }

    fn read(&mut self) -> Result<Selection, SyncError> {
        let text = fs::read_to_string(&self.path).map_err(|source| SyncError::Io {
            path: self.path.clone(),
            source,
        })?;
        if self.text.as_ref() != Some(&text) {
            self.selection = match Config::from_yaml(&text) {
                Ok(config) => config.theme.map(Selection::Named).unwrap_or_default(),
                Err(err @ ConfigError::InvalidThemeSelector) => {
                    Selection::Invalid(format!("{}: {err}", self.path.display()))
                }
                Err(source) => {
                    return Err(SyncError::Config {
                        path: self.path.clone(),
                        source,
                    });
                }
            };
            self.text = Some(text);
        }
        Ok(self.selection.clone())
    }
}
