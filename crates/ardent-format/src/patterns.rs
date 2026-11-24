//! Gitignore patterns used by format manifests and project configuration.

use std::collections::BTreeMap;
use std::path::Path;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("invalid pattern: {0}")]
pub struct PatternError(#[from] ignore::Error);

pub fn compile(root: &Path, patterns: &[String]) -> Result<Gitignore, PatternError> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        builder.add_line(None, pattern)?;
    }
    Ok(builder.build()?)
}

/// Validate every default and return the matcher's file selection for this format.
pub fn compile_defaults(
    root: &Path,
    files: &[String],
    prefixes: &BTreeMap<String, Vec<String>>,
) -> Result<Gitignore, PatternError> {
    for patterns in prefixes.values() {
        compile(root, patterns)?;
    }
    compile(root, files)
}
