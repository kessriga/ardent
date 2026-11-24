//! Loads project-installed format components and validates their output before Ardent builds an outline.

mod package;
mod runtime;

pub use package::{FormatPlugin, PluginError, check_installed_names};
pub use runtime::RuntimeError;
