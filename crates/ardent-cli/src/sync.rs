//! `ardent sync`: brings the structure files in `.ardent/cache/` up to date with the tracked source files (spec `sync`,
//! decision-2). M1 has no views, so a sync has no links to re-resolve.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use ardent_core::config::{self, Config, ConfigError};
use ardent_core::layout::{self, ARDENT_DIR, CACHE_DIR, FILES_DIR, LOCK_FILE, STRUCTURE_FILE};
use ardent_core::outline::Outline;
use ardent_core::structure::{self, Stamp, Structure};
use ardent_format::engine::{FormatPlugin, PluginError};
use ardent_format::markdown;

use crate::project_root;
use crate::tracked::{self, FormatPatterns, PatternError};

/// Why a sync did not run. Nothing in `.ardent/` changed, except that the lock file may exist.
pub enum SyncError {
    NoProject(PathBuf),
    Io { path: PathBuf, source: io::Error },
    Config { path: PathBuf, source: ConfigError },
    Pattern { path: PathBuf, source: PatternError },
    Plugin { name: String, source: PluginError },
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoProject(dir) => write!(
                f,
                "{} is not inside an Ardent project; run `ardent init` in the project root first",
                dir.display()
            ),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Config { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Pattern { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Plugin { name, source } => write!(f, "built-in format plugin {name}: {source}"),
        }
    }
}

/// What a sync did. `warnings` and `errors` are messages for standard error; an error means that the sync is
/// incomplete.
#[derive(Default)]
pub struct Report {
    pub tracked: usize,
    pub scanned: usize,
    pub removed: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

enum Outcome {
    Unchanged,
    Restamped,
    Scanned,
}

/// Runs a sync in the project of `working_dir`.
pub fn run(working_dir: &Path) -> Result<Report, SyncError> {
    let root = find_root(working_dir)?;
    let ardent_dir = root.join(ARDENT_DIR);
    let config = read_config(&ardent_dir)?;
    let plugins = builtin_plugins()?;
    let mut report = Report::default();
    let formats = enabled_formats(&config, &plugins, &mut report.warnings);
    let _lock = lock(&ardent_dir)?;
    let walk = tracked::walk(&root, &formats.patterns(), &config.exclude).map_err(|source| {
        SyncError::Pattern {
            path: ardent_dir.join(config::FILE_NAME),
            source,
        }
    })?;
    report
        .warnings
        .extend(walk.warnings.iter().map(ToString::to_string));
    report.errors.extend(walk.errors.iter().cloned());
    report.tracked = walk.files.len();
    for file in &walk.files {
        let plugin = formats.plugins[file.format];
        match sync_file(&root, &ardent_dir, &file.path, plugin) {
            Ok(Outcome::Scanned) => report.scanned += 1,
            Ok(Outcome::Unchanged | Outcome::Restamped) => {}
            Err(err) => report.errors.push(format!("{}: {err}", file.path)),
        }
    }
    // An incomplete walk lists too few files, so it must not delete the structure of the files it missed.
    if walk.errors.is_empty() {
        let tracked: HashSet<&str> = walk.files.iter().map(|file| file.path.as_str()).collect();
        let mut pruner = Pruner {
            tracked: &tracked,
            removed: 0,
            errors: &mut report.errors,
        };
        pruner.prune(&ardent_dir.join(FILES_DIR), "");
        report.removed = pruner.removed;
    }
    Ok(report)
}

fn find_root(working_dir: &Path) -> Result<PathBuf, SyncError> {
    match project_root::find(working_dir) {
        Ok(Some(root)) => Ok(root),
        Ok(None) => Err(SyncError::NoProject(working_dir.to_path_buf())),
        Err((path, source)) => Err(SyncError::Io { path, source }),
    }
}

fn read_config(ardent_dir: &Path) -> Result<Config, SyncError> {
    let path = ardent_dir.join(config::FILE_NAME);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) => return Err(SyncError::Io { path, source }),
    };
    Config::from_yaml(&text).map_err(|source| SyncError::Config { path, source })
}

fn builtin_plugins() -> Result<Vec<FormatPlugin>, SyncError> {
    let markdown = markdown::plugin().map_err(|source| SyncError::Plugin {
        name: markdown::NAME.to_string(),
        source,
    })?;
    Ok(vec![markdown])
}

/// The enabled formats that have a plugin, with their file patterns. `TrackedFile::format` indexes both lists.
struct EnabledFormats<'a> {
    plugins: Vec<&'a FormatPlugin>,
    files: Vec<&'a [String]>,
}

impl EnabledFormats<'_> {
    fn patterns(&self) -> Vec<FormatPatterns<'_>> {
        self.plugins
            .iter()
            .zip(&self.files)
            .map(|(plugin, files)| FormatPatterns {
                name: &plugin.name,
                files,
            })
            .collect()
    }
}

/// Picks the plugin of each format that `config.yaml` enables, and warns about each format without a plugin.
fn enabled_formats<'a>(
    config: &'a Config,
    plugins: &'a [FormatPlugin],
    warnings: &mut Vec<String>,
) -> EnabledFormats<'a> {
    let mut enabled = EnabledFormats {
        plugins: Vec::new(),
        files: Vec::new(),
    };
    for (name, settings) in &config.formats {
        let Some(plugin) = plugins.iter().find(|plugin| &plugin.name == name) else {
            warnings.push(format!(
                "the format `{name}` has no plugin; Ardent tracks no file for it"
            ));
            continue;
        };
        enabled.plugins.push(plugin);
        enabled
            .files
            .push(settings.files.as_deref().unwrap_or(&plugin.default_files));
    }
    enabled
}

/// Takes the lock of the spec `ardent-layout`, which the operating system releases when the process ends.
fn lock(ardent_dir: &Path) -> Result<File, SyncError> {
    let io_error = |path: PathBuf| move |source| SyncError::Io { path, source };
    let cache_dir = ardent_dir.join(CACHE_DIR);
    fs::create_dir_all(&cache_dir).map_err(io_error(cache_dir))?;
    let path = ardent_dir.join(LOCK_FILE);
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(io_error(path.clone()))?;
    file.lock().map_err(io_error(path))?;
    Ok(file)
}

/// Brings the structure file of the tracked file `path` up to date: keeps it when the stamp matches, updates only the
/// stamp when the content hash matches, and scans the file otherwise.
fn sync_file(
    root: &Path,
    ardent_dir: &Path,
    path: &str,
    plugin: &FormatPlugin,
) -> Result<Outcome, Box<dyn Error>> {
    let mut file = File::open(root.join(path))?;
    let metadata = file.metadata()?;
    let (size, mtime) = (metadata.len(), metadata.modified()?);
    let structure_path = layout::structure_path(path);
    let previous = read_structure(&ardent_dir.join(&structure_path))
        .filter(|old| built_by(&old.stamp, plugin));
    if previous
        .as_ref()
        .is_some_and(|old| old.stamp.size == size && old.stamp.mtime == mtime)
    {
        return Ok(Outcome::Unchanged);
    }
    let mut source = Vec::new();
    file.read_to_end(&mut source)?;
    let stamp = Stamp {
        size,
        mtime,
        hash: structure::content_hash(&source),
        format: plugin.name.clone(),
        format_version: plugin.version.clone(),
        format_hash: plugin.format_hash.clone(),
    };
    let (outline, outcome) = match previous {
        Some(old) if old.stamp.hash == stamp.hash => (old.outline, Outcome::Restamped),
        _ => (
            Outline::build(path, &source, &plugin.scan(&source)?),
            Outcome::Scanned,
        ),
    };
    let yaml = Structure::new(stamp, outline).to_yaml()?;
    layout::write_atomic(ardent_dir, &structure_path, yaml.as_bytes())?;
    Ok(outcome)
}

/// The structure file at `path`, or `None` when it is missing or cannot be read, so that the sync builds it again.
fn read_structure(path: &Path) -> Option<Structure> {
    let text = fs::read_to_string(path).ok()?;
    Structure::from_yaml(&text).ok()
}

fn built_by(stamp: &Stamp, plugin: &FormatPlugin) -> bool {
    stamp.format == plugin.name
        && stamp.format_version == plugin.version
        && stamp.format_hash == plugin.format_hash
}

/// Deletes the structure files of source files that are not tracked, and the directories under `cache/files/` that
/// become empty.
struct Pruner<'a> {
    tracked: &'a HashSet<&'a str>,
    removed: usize,
    errors: &'a mut Vec<String>,
}

impl Pruner<'_> {
    /// Prunes `dir`, the directory of `source_path` under `cache/files/`, or `cache/files/` itself when `source_path`
    /// is empty.
    fn prune(&mut self, dir: &Path, source_path: &str) {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return,
            Err(err) => return self.errors.push(format!("{}: {err}", dir.display())),
        };
        for entry in entries {
            match entry.and_then(|entry| Ok((entry.file_type()?, entry))) {
                Ok((file_type, entry)) => self.prune_entry(file_type, &entry.path(), source_path),
                Err(err) => self.errors.push(format!("{}: {err}", dir.display())),
            }
        }
        if !source_path.is_empty() {
            // Fails, as intended, when the directory still holds files.
            let _ = fs::remove_dir(dir);
        }
    }

    fn prune_entry(&mut self, file_type: fs::FileType, path: &Path, source_path: &str) {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return;
        };
        if file_type.is_dir() {
            let child = match source_path {
                "" => name.to_string(),
                parent => format!("{parent}/{name}"),
            };
            self.prune(path, &child);
        } else if name == STRUCTURE_FILE && !self.tracked.contains(source_path) {
            match fs::remove_file(path) {
                Ok(()) => self.removed += 1,
                Err(err) => self.errors.push(format!("{}: {err}", path.display())),
            }
        }
    }
}
