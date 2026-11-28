//! The project configuration `.ardent/config.yaml` (spec `ardent-layout`): which format and view plugins run, their
//! file patterns, and which files Ardent never tracks.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Deserializer};

use crate::yaml::{self, ReadError};

/// The schema version of `config.yaml` that this Ardent reads. It also versions the layout of `.ardent/`.
pub const SCHEMA: u32 = 1;
/// The name of the configuration file, relative to `.ardent/`.
pub const FILE_NAME: &str = "config.yaml";
/// A plugin name that no plugin can have, because `structure.yaml` shares a directory with derived view files.
const RESERVED_PLUGIN_NAME: &str = "structure";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    schema: u32,
    /// The enabled format plugins by name.
    #[serde(default)]
    pub formats: BTreeMap<String, FormatConfig>,
    /// The enabled view plugins by name.
    #[serde(default)]
    pub views: BTreeMap<String, ViewConfig>,
    /// Gitignore-style patterns of files that Ardent never tracks.
    #[serde(default, deserialize_with = "specified")]
    pub exclude: Option<Vec<String>>,
    /// Where sync reuses compiled format components. An omitted setting inherits the user's choice.
    #[serde(default, deserialize_with = "specified")]
    pub compilation_cache: Option<CompilationCache>,
    /// Name of the user-installed palette to display in the browser.
    #[serde(default, deserialize_with = "specified")]
    pub theme: Option<String>,
}

// An omitted key inherits, while an explicit value must have the declared type.
fn specified<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    input: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(input).map(Some)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompilationCache {
    #[default]
    User,
    Project,
}

/// Settings of one enabled format plugin. A key that is absent keeps the default of the plugin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatConfig {
    /// Gitignore-style patterns of the files that the plugin parses.
    pub files: Option<Vec<String>>,
    /// Search prefixes, each with the patterns of the files it covers.
    pub prefixes: Option<BTreeMap<String, Vec<String>>>,
}

/// Settings of one enabled view plugin. Schema version 1 has none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewConfig {}

/// Why Ardent cannot use a `config.yaml`.
#[derive(Debug)]
pub enum ConfigError {
    Read(ReadError),
    /// A plugin name breaks the rule of the spec `ardent-layout`, "Plugin names".
    InvalidPluginName(String),
    /// The configuration enables a plugin with the reserved name `structure`.
    ReservedPluginName,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => err.fmt(f),
            Self::InvalidPluginName(name) => write!(
                f,
                "{name:?} is not a valid plugin name: a name starts with a lowercase letter and contains only \
                 lowercase letters, digits, and hyphens"
            ),
            Self::ReservedPluginName => {
                write!(f, "the plugin name `{RESERVED_PLUGIN_NAME}` is reserved")
            }
        }
    }
}

impl Error for ConfigError {}

impl Config {
    /// Fill absent project settings for the formats the project enables.
    pub fn inherit(&mut self, user: Self) {
        self.exclude = self.exclude.take().or(user.exclude);
        self.compilation_cache = self.compilation_cache.or(user.compilation_cache);
        self.theme = self.theme.take().or(user.theme);
        for (name, defaults) in user.formats {
            if let Some(format) = self.formats.get_mut(&name) {
                format.files = format.files.take().or(defaults.files);
                if let Some(mut prefixes) = defaults.prefixes {
                    prefixes.extend(format.prefixes.take().unwrap_or_default());
                    format.prefixes = Some(prefixes);
                }
            }
        }
    }

    pub fn from_yaml(text: &str) -> Result<Self, ConfigError> {
        let config: Self = yaml::read(text, SCHEMA).map_err(ConfigError::Read)?;
        for name in config.formats.keys().chain(config.views.keys()) {
            check_plugin_name(name)?;
        }
        Ok(config)
    }
}

/// Checks that a plugin name is valid for configuration and project package lookup.
pub fn check_plugin_name(name: &str) -> Result<(), ConfigError> {
    let mut chars = name.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    match (valid, name == RESERVED_PLUGIN_NAME) {
        (false, _) => Err(ConfigError::InvalidPluginName(name.to_string())),
        (true, true) => Err(ConfigError::ReservedPluginName),
        (true, false) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Projects created by earlier versions of `ardent init` have explicit file patterns.
    const INITIAL: &str = r#"schema: 1
formats:
  markdown:
    files:
      - "*.md"
      - "*.markdown"
  rust:
    files:
      - "*.rs"
views: {}
exclude: []
"#;

    fn patterns(patterns: &[&str]) -> Option<Vec<String>> {
        Some(patterns.iter().map(|p| p.to_string()).collect())
    }

    #[test]
    fn reads_the_initial_configuration() -> Result<(), ConfigError> {
        let config = Config::from_yaml(INITIAL)?;
        let names: Vec<_> = config.formats.keys().collect();
        assert_eq!(names, ["markdown", "rust"]);
        assert_eq!(
            config.formats["markdown"].files,
            patterns(&["*.md", "*.markdown"])
        );
        assert_eq!(config.formats["rust"].prefixes, None);
        assert!(config.views.is_empty() && config.exclude == Some(vec![]));
        Ok(())
    }

    #[test]
    fn reads_prefixes_views_and_excludes() -> Result<(), ConfigError> {
        let text = "schema: 1\nformats:\n  markdown:\n    prefixes:\n      spec:\n        - docs/**/*.md\n\
                    views:\n  comments: {}\nexclude:\n  - vendor/\n";
        let config = Config::from_yaml(text)?;
        let markdown = &config.formats["markdown"];
        assert_eq!(markdown.files, None);
        assert_eq!(
            markdown.prefixes.as_ref().map(|p| p["spec"].clone()),
            patterns(&["docs/**/*.md"])
        );
        assert!(config.views.contains_key("comments"));
        assert_eq!(config.exclude, patterns(&["vendor/"]));
        Ok(())
    }

    #[test]
    fn inherits_settings_only_for_enabled_formats() -> Result<(), ConfigError> {
        let user = Config::from_yaml(
            "schema: 1\ncompilation_cache: project\nexclude: [vendor/]\nformats: {rust: {}, markdown: {files: ['*.md'], prefixes: {a: [old], b: [keep]}}}",
        )?;
        let mut project =
            Config::from_yaml("schema: 1\nformats: {markdown: {prefixes: {a: [new]}}}")?;
        project.inherit(user);
        assert_eq!(project.compilation_cache, Some(CompilationCache::Project));
        assert_eq!(project.exclude, patterns(&["vendor/"]));
        assert_eq!(project.formats.len(), 1);
        let format = &project.formats["markdown"];
        assert_eq!(format.files, patterns(&["*.md"]));
        let prefixes = format.prefixes.as_ref().expect("merged prefix settings");
        assert_eq!(prefixes["a"], ["new"]);
        assert_eq!(prefixes["b"], ["keep"]);
        Ok(())
    }

    #[test]
    fn rejects_a_missing_or_unknown_schema() {
        let missing = INITIAL.replace("schema: 1\n", "");
        assert!(matches!(
            Config::from_yaml(&missing),
            Err(ConfigError::Read(ReadError::UnknownSchema(None)))
        ));
        let newer = INITIAL.replace("schema: 1", "schema: 2");
        assert!(matches!(
            Config::from_yaml(&newer),
            Err(ConfigError::Read(ReadError::UnknownSchema(Some(2))))
        ));
    }

    #[test]
    fn rejects_an_unknown_key() {
        let typo = INITIAL.replace("exclude:", "exlude:");
        let message = Config::from_yaml(&typo).err().map(|err| err.to_string());
        assert!(message.is_some_and(|m| m.contains("exlude")));
    }

    #[test]
    fn rejects_invalid_and_reserved_plugin_names() {
        let reserved = INITIAL.replace("views: {}", "views:\n  structure: {}");
        assert!(matches!(
            Config::from_yaml(&reserved),
            Err(ConfigError::ReservedPluginName)
        ));
        for name in ["Markdown", "1md", "mark_down", ""] {
            let text = INITIAL.replace("  rust:", &format!("  {name:?}:"));
            assert!(
                matches!(
                    Config::from_yaml(&text),
                    Err(ConfigError::InvalidPluginName(_))
                ),
                "{name:?}"
            );
        }
    }
}
