//! Poll only the selected user palette. Keep the last usable colors during an incomplete save.

use std::io;
use std::path::{Path, PathBuf};

use ardent_core::theme::Theme;

use crate::sync::source;

#[derive(Default)]
pub(super) struct ActiveTheme {
    pub name: Option<String>,
    input: Option<Result<(PathBuf, String), String>>,
    pub colors: Theme,
    pub diagnostic: Option<String>,
}

impl ActiveTheme {
    /// Compare the actual bytes read, so atomic saves and edits with identical timestamps are visible.
    pub fn refresh(&mut self, user_dir: Option<&Path>) -> bool {
        let input = self.name.as_deref().map(|name| read(user_dir, name));
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
        self.diagnostic = match parsed {
            Ok(colors) => {
                self.colors = colors;
                None
            }
            Err(message) => Some(message),
        };
        true
    }
}

fn read(user_dir: Option<&Path>, name: &str) -> Result<(PathBuf, String), String> {
    if matches!(name, "" | "." | "..") || name.contains(['/', '\\', ':', '\0']) {
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
        for name in ["", ".", "..", "/tmp/a", "../a", "a/b", "a\\b", "C:relative"] {
            assert!(
                read(None, name)
                    .expect_err("invalid stem")
                    .contains("invalid theme name")
            );
        }
    }
}
