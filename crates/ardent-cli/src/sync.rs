//! `ardent sync`: brings the structure files in `.ardent/cache/` up to date with the tracked source files (spec `sync`,
//! decision-2). M1 has no views, so a sync has no links to re-resolve.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ardent_core::config::{self, Config, ConfigError};
use ardent_core::layout::{self, ARDENT_DIR, CACHE_DIR, FILES_DIR, LOCK_FILE, STRUCTURE_FILE};
use ardent_core::outline::Outline;
use ardent_core::structure::{self, Stamp, Structure};
use ardent_format::{self as format, FormatPlugin};
use ignore::gitignore::Gitignore;

use crate::project_root;
use crate::tracked::{self, PatternError, Patterns, TrackedFile};

/// Why a sync did not run. Nothing in `.ardent/` changed, except that the lock file may exist.
pub enum SyncError {
    NoProject(PathBuf),
    Io {
        path: PathBuf,
        source: io::Error,
    },
    /// A directory in `.ardent/` could not be created. The error names it.
    CreateDir(io::Error),
    Config {
        path: PathBuf,
        source: ConfigError,
    },
    Pattern {
        path: PathBuf,
        source: PatternError,
    },
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
            Self::CreateDir(source) => write!(f, "{source}"),
            Self::Config { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Pattern { path, source } => write!(f, "{}: {source}", path.display()),
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
    let config_error = |source| SyncError::Pattern {
        path: ardent_dir.join(config::FILE_NAME),
        source,
    };
    let mut patterns = Patterns::new(&root, &config.exclude).map_err(config_error)?;
    let overrides = configured_patterns(&root, &config).map_err(config_error)?;
    let _lock = lock(&ardent_dir)?;
    let mut report = Report::default();
    if let Err(err) = format::check_installed_names(&ardent_dir) {
        report.errors.push(err.to_string());
    }
    let (mut plugins, unavailable) =
        load_plugins(&root, &ardent_dir, overrides, &mut patterns, &mut report);
    let files_dir = create_dir(&ardent_dir, FILES_DIR)?;
    let walk = tracked::walk(&root, &patterns);
    report
        .warnings
        .extend(walk.warnings.iter().map(ToString::to_string));
    report.errors.extend(walk.errors.iter().cloned());
    report.tracked = walk.files.len();
    // A failed package or walk can omit tracked files. Prune only after complete discovery,
    // before scans reuse paths of case-only renames.
    if report.errors.is_empty() {
        report.removed = prune(&files_dir, &walk.files, &mut report.errors);
    }
    for file in &walk.files {
        let plugin = &mut plugins[file.format];
        match sync_file(&root, &ardent_dir, &file.path, plugin, &unavailable) {
            Ok(Outcome::Scanned) => report.scanned += 1,
            Ok(Outcome::Unchanged | Outcome::Restamped) => {}
            Err(err) => report
                .errors
                .push(format!("{}: {err} (format {})", file.path, plugin.name)),
        }
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

struct ConfiguredFormat<'a> {
    name: &'a str,
    files: Option<Gitignore>,
}

/// Validate project overrides before creating the lock or verification records.
fn configured_patterns<'a>(
    root: &Path,
    config: &'a Config,
) -> Result<Vec<ConfiguredFormat<'a>>, PatternError> {
    let mut overrides = Vec::new();
    for (name, settings) in &config.formats {
        if let Some(prefixes) = &settings.prefixes {
            for files in prefixes.values() {
                tracked::matcher(root, files)?;
            }
        }
        let files = settings
            .files
            .as_ref()
            .map(|files| tracked::matcher(root, files))
            .transpose()?;
        overrides.push(ConfiguredFormat { name, files });
    }
    Ok(overrides)
}

fn load_plugins<'a>(
    root: &Path,
    ardent_dir: &Path,
    overrides: Vec<ConfiguredFormat<'a>>,
    patterns: &mut Patterns<'a>,
    report: &mut Report,
) -> (Vec<FormatPlugin>, HashSet<&'a str>) {
    let mut plugins = Vec::new();
    let mut unavailable = HashSet::new();
    for ConfiguredFormat { name, files } in overrides {
        let plugin = match FormatPlugin::load(ardent_dir, name) {
            Ok(Some(plugin)) => plugin,
            Ok(None) => {
                report.warnings.push(format!(
                    "the format `{name}` has no plugin; Ardent tracks no file for it"
                ));
                continue;
            }
            Err(err) => {
                unavailable.insert(name);
                report.errors.push(format!("format plugin {name}: {err}"));
                continue;
            }
        };
        match plugin_patterns(root, &plugin) {
            Ok(defaults) => {
                patterns.add(name, files.unwrap_or(defaults));
                plugins.push(plugin);
            }
            Err(err) => {
                unavailable.insert(name);
                report.errors.push(format!("format plugin {name}: {err}"));
            }
        }
    }
    (plugins, unavailable)
}

fn plugin_patterns(root: &Path, plugin: &FormatPlugin) -> Result<Gitignore, PatternError> {
    for files in plugin.default_prefixes.values() {
        tracked::matcher(root, files)?;
    }
    tracked::matcher(root, &plugin.default_files)
}

/// Takes the lock of the spec `ardent-layout`, which the operating system releases when the process ends.
fn lock(ardent_dir: &Path) -> Result<File, SyncError> {
    let io_error = |path: PathBuf| move |source| SyncError::Io { path, source };
    create_dir(ardent_dir, CACHE_DIR)?;
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

/// Creates the directory `relative_path` in `.ardent/`, and fails when a part of it is a symlink.
fn create_dir(ardent_dir: &Path, relative_path: &str) -> Result<PathBuf, SyncError> {
    layout::create_dir(ardent_dir, relative_path).map_err(SyncError::CreateDir)
}

/// Brings the structure file of the tracked file `path` up to date: keeps it when the stamp matches, updates only the
/// stamp when the content hash matches, and scans the file otherwise.
fn sync_file(
    root: &Path,
    ardent_dir: &Path,
    path: &str,
    plugin: &mut FormatPlugin,
    unavailable: &HashSet<&str>,
) -> Result<Outcome, Box<dyn Error>> {
    let mut file = File::open(root.join(path))?;
    let metadata = file.metadata()?;
    let (size, mtime) = (metadata.len(), metadata.modified()?);
    let structure_path = layout::structure_path(path);
    let previous = read_structure(&ardent_dir.join(&structure_path));
    // Incomplete discovery can hide a pattern conflict with the previous owner.
    if previous
        .as_ref()
        .is_some_and(|(old, _)| unavailable.contains(old.stamp.format.as_str()))
    {
        return Ok(Outcome::Unchanged);
    }
    let previous = previous.filter(|(old, _)| built_by(&old.stamp, plugin));
    if previous
        .as_ref()
        .is_some_and(|(old, written)| is_unchanged(&old.stamp, size, mtime, *written))
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
        Some((old, _)) if old.stamp.hash == stamp.hash => (old.outline, Outcome::Restamped),
        _ => (
            Outline::build(path, &source, &plugin.scan(path, &source)?),
            Outcome::Scanned,
        ),
    };
    let yaml = Structure::new(stamp, outline).to_yaml()?;
    layout::write_atomic(ardent_dir, &structure_path, yaml.as_bytes())?;
    Ok(outcome)
}

/// The structure file at `path` and the time it was written, or `None` when it is missing or cannot be read, so that
/// the sync builds it again.
fn read_structure(path: &Path) -> Option<(Structure, SystemTime)> {
    let text = fs::read_to_string(path).ok()?;
    let written = fs::metadata(path).and_then(|m| m.modified()).ok()?;
    Some((Structure::from_yaml(&text).ok()?, written))
}

/// Whether a source file with `size` and `mtime` still has the content of `stamp`, without reading it. A file whose
/// mtime is not older than its structure file may have changed again within one tick of a coarse clock, so the sync
/// compares its content hash.
fn is_unchanged(stamp: &Stamp, size: u64, mtime: SystemTime, written: SystemTime) -> bool {
    stamp.size == size && stamp.mtime == mtime && mtime < written
}

fn built_by(stamp: &Stamp, plugin: &FormatPlugin) -> bool {
    stamp.format == plugin.name
        && stamp.format_version == plugin.version
        && stamp.format_hash == plugin.format_hash
}

/// Deletes the structure files in `files_dir` of source files that are not tracked, and returns how many.
fn prune(files_dir: &Path, files: &[TrackedFile], errors: &mut Vec<String>) -> usize {
    let tracked: HashSet<&str> = files.iter().map(|file| file.path.as_str()).collect();
    let mut pruner = Pruner {
        tracked: &tracked,
        removed: 0,
        errors,
    };
    pruner.prune(files_dir, "");
    pruner.removed
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
