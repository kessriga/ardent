//! Loads project-installed format components and validates their output before Ardent builds an outline.

mod package;
pub mod patterns;
mod runtime;

pub use package::{FormatPlugin, InstalledPackages, PluginError};
pub use runtime::{MAX_SOURCE_BYTES, RuntimeError};
