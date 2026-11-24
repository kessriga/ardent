use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::str;

use ardent_core::config::{self, ConfigError};
use ardent_core::layout;
use ardent_core::outline::ScannedNode;
use ardent_core::structure;
use ardent_core::yaml::{self, ReadError};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::runtime::{INTERFACE, Runtime, RuntimeError};

const MAX_MANIFEST_BYTES: u64 = 1 << 20;
const MAX_COMPONENT_BYTES: u64 = 64 << 20;
const VERIFIED_DIR: &str = "cache/verified";

/// An installed package failed a read or verification. Other formats can still run.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error(transparent)]
    Name(#[from] ConfigError),
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("{path}: {source}")]
    Manifest { path: PathBuf, source: ReadError },
    #[error("{path}: {reason}")]
    Invalid { path: PathBuf, reason: String },
    #[error("{path}: {source}")]
    Component { path: PathBuf, source: RuntimeError },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    #[serde(rename = "schema")]
    _schema: u32,
    name: String,
    version: String,
    kind: String,
    component: PathBuf,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    prefixes: BTreeMap<String, Vec<String>>,
}

impl Manifest {
    fn read(path: &Path, name: &str) -> Result<(Self, String), PluginError> {
        let (manifest, manifest_bytes): (Self, _) = read_manifest(path)?;
        check_manifest_name(path, &manifest.name, name)?;
        if manifest.kind != "format" {
            return Err(invalid(
                path,
                format!("expected kind format, found {:?}", manifest.kind),
            ));
        }
        Ok((manifest, structure::content_hash(&manifest_bytes)))
    }
}

fn read_manifest<T: DeserializeOwned>(path: &Path) -> Result<(T, Vec<u8>), PluginError> {
    let bytes = read_file(path, MAX_MANIFEST_BYTES)?;
    let text = str::from_utf8(&bytes).map_err(|err| invalid(path, err.to_string()))?;
    let value = yaml::read(text, 1).map_err(|source| PluginError::Manifest {
        path: path.to_owned(),
        source,
    })?;
    Ok((value, bytes))
}

fn check_manifest_name(path: &Path, declared: &str, directory: &str) -> Result<(), PluginError> {
    if declared != directory {
        return Err(invalid(
            path,
            format!("name {declared:?} differs from directory name {directory:?}"),
        ));
    }
    Ok(())
}

/// Directory entries as spelled on disk, including on case-insensitive filesystems.
pub struct InstalledPackages<'a> {
    ardent_dir: &'a Path,
    names: BTreeSet<OsString>,
}

impl<'a> InstalledPackages<'a> {
    pub fn discover(ardent_dir: &'a Path) -> Result<Self, PluginError> {
        let plugins = ardent_dir.join("plugins");
        let mut names = BTreeSet::new();
        if directory_exists(&plugins)? {
            for entry in fs::read_dir(&plugins).map_err(|err| io_error(&plugins, err))? {
                names.insert(entry.map_err(|err| io_error(&plugins, err))?.file_name());
            }
        }
        Ok(Self { ardent_dir, names })
    }

    /// Enabled formats check identity during loading. Other installed packages still need matching names.
    pub fn check_other_names(&self, enabled_format: impl Fn(&str) -> bool) -> Vec<PluginError> {
        self.names
            .iter()
            .filter(|name| !name.to_str().is_some_and(&enabled_format))
            .filter_map(|name| self.check_other_name(name).err())
            .collect()
    }

    fn check_other_name(&self, name: &OsStr) -> Result<(), PluginError> {
        #[derive(Deserialize)]
        struct Identity {
            name: String,
        }

        let dir = self.ardent_dir.join("plugins").join(name);
        if !fs::symlink_metadata(&dir)
            .map_err(|err| io_error(&dir, err))?
            .is_dir()
        {
            return Ok(());
        }
        let path = dir.join("plugin.yaml");
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() => return Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(io_error(&path, err)),
            Ok(_) => {}
        }
        let name = name
            .to_str()
            .ok_or_else(|| invalid(&dir, "plugin names must be ASCII"))?;
        config::check_plugin_name(name).map_err(|err| invalid(&path, err.to_string()))?;
        let (identity, _): (Identity, _) = read_manifest(&path)?;
        check_manifest_name(&path, &identity.name, name)
    }

    /// Loads an enabled format under the project's sync lock. A missing directory gives `Ok(None)`.
    pub fn load_format(&self, name: &str) -> Result<Option<FormatPlugin>, PluginError> {
        config::check_plugin_name(name)?;
        let dir = self.ardent_dir.join("plugins").join(name);
        if !directory_exists(&dir)? {
            return Ok(None);
        }
        if !self.names.contains(OsStr::new(name)) {
            return Err(invalid(
                &dir,
                format!("package directory spelling must match {name:?} exactly"),
            ));
        }
        FormatPlugin::load_directory(self.ardent_dir, &dir, name).map(Some)
    }
}

enum Scanner {
    Verified(Vec<u8>),
    Active(Box<Runtime>),
}

/// A verified project package. The stamp and the running scanner use the same component bytes.
pub struct FormatPlugin {
    pub name: String,
    pub version: String,
    pub format_hash: String,
    pub default_files: Vec<String>,
    pub default_prefixes: BTreeMap<String, Vec<String>>,
    scanner: Scanner,
}

impl FormatPlugin {
    /// Loads an enabled format under the project's sync lock. A missing package directory gives `Ok(None)`.
    pub fn load(ardent_dir: &Path, name: &str) -> Result<Option<Self>, PluginError> {
        InstalledPackages::discover(ardent_dir)?.load_format(name)
    }

    fn load_directory(ardent_dir: &Path, dir: &Path, name: &str) -> Result<Self, PluginError> {
        let path = dir.join("plugin.yaml");
        let (manifest, manifest_hash) = Manifest::read(&path, name)?;
        let component_path = package_file(dir, &manifest.component)?;
        let bytes = read_file(&component_path, MAX_COMPONENT_BYTES)?;
        let format_hash = structure::content_hash(&bytes);
        let record = format!(
            "schema: 1\ninterface: {INTERFACE}\nmanifest_hash: {manifest_hash}\ncomponent_hash: {format_hash}\n"
        );
        let cache_dir = layout::create_dir(ardent_dir, VERIFIED_DIR)
            .map_err(|source| io_error(ardent_dir.join(VERIFIED_DIR), source))?;
        let filename = format!("{name}.yaml");
        let record_path = cache_dir.join(&filename);
        let scanner = if record_matches(&record_path, &record)? {
            Scanner::Verified(bytes)
        } else {
            let runtime = verify_component(&bytes, &manifest, &component_path, &path)?;
            layout::write_atomic(
                ardent_dir,
                &format!("{VERIFIED_DIR}/{filename}"),
                record.as_bytes(),
            )
            .map_err(|source| io_error(record_path, source))?;
            Scanner::Active(Box::new(runtime))
        };
        Ok(Self {
            name: manifest.name,
            version: manifest.version,
            default_files: manifest.files,
            default_prefixes: manifest.prefixes,
            format_hash,
            scanner,
        })
    }

    /// Scans with the verified component, compiling on first use after a verification cache hit.
    pub fn scan(&mut self, path: &str, source: &[u8]) -> Result<Vec<ScannedNode>, RuntimeError> {
        match &mut self.scanner {
            Scanner::Active(runtime) => runtime.scan(path, source),
            Scanner::Verified(bytes) => {
                let mut runtime = Runtime::compile(bytes)?;
                let result = runtime.scan(path, source);
                self.scanner = Scanner::Active(Box::new(runtime));
                result
            }
        }
    }
}

fn verify_component(
    bytes: &[u8],
    manifest: &Manifest,
    component_path: &Path,
    manifest_path: &Path,
) -> Result<Runtime, PluginError> {
    let component_error = |source| PluginError::Component {
        path: component_path.to_path_buf(),
        source,
    };
    let mut runtime = Runtime::compile(bytes).map_err(component_error)?;
    let metadata = runtime.describe().map_err(component_error)?;
    if metadata.name != manifest.name
        || metadata.version != manifest.version
        || metadata.files != manifest.files
        || metadata.prefixes != manifest.prefixes
    {
        return Err(invalid(
            manifest_path,
            "name, version, files or prefixes differ from component describe",
        ));
    }
    Ok(runtime)
}

/// A manifest can name a file in a subdirectory. Each parent must be a real directory inside the package.
fn package_file(dir: &Path, relative: &Path) -> Result<PathBuf, PluginError> {
    let mut parts = relative.components().peekable();
    let mut path = dir.to_owned();
    if parts.peek().is_none() {
        return Err(invalid(dir, "empty component path"));
    }
    while let Some(part) = parts.next() {
        if part == Component::CurDir {
            continue;
        }
        let Component::Normal(name) = part else {
            return Err(invalid(
                dir,
                format!("component path {relative:?} must be inside the package"),
            ));
        };
        path.push(name);
        if parts.peek().is_some() && !directory_exists(&path)? {
            return Err(io_error(path, io::ErrorKind::NotFound.into()));
        }
    }
    Ok(path)
}

fn directory_exists(path: &Path) -> Result<bool, PluginError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(_) => Err(invalid(
            path,
            "expected a directory, found a symlink or another file type",
        )),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(io_error(path, err)),
    }
}

fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>, PluginError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.is_file() {
        return Err(invalid(path, "expected a regular file"));
    }
    let too_large = || invalid(path, format!("file exceeds the {limit}-byte limit"));
    if metadata.len() > limit {
        return Err(too_large());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .and_then(|file| file.take(limit + 1).read_to_end(&mut bytes))
        .map_err(|source| io_error(path, source))?;
    if bytes.len() as u64 > limit {
        return Err(too_large());
    }
    Ok(bytes)
}

fn record_matches(path: &Path, expected: &str) -> Result<bool, PluginError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() => {
            Err(invalid(path, "expected a regular verification record"))
        }
        Ok(metadata) if metadata.len() == expected.len() as u64 => {
            Ok(fs::read(path).map_err(|source| io_error(path, source))? == expected.as_bytes())
        }
        Ok(_) => Ok(false),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(io_error(path, err)),
    }
}

fn io_error(path: impl Into<PathBuf>, source: io::Error) -> PluginError {
    PluginError::Io {
        path: path.into(),
        source,
    }
}

fn invalid(path: &Path, reason: impl Into<String>) -> PluginError {
    PluginError::Invalid {
        path: path.to_owned(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io::Write;

    use super::*;

    type TestResult = Result<(), Box<dyn Error>>;

    fn install_rust(ardent_dir: &Path) -> io::Result<PathBuf> {
        let package = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/rust");
        let installed = ardent_dir.join("plugins/rust");
        fs::create_dir_all(&installed)?;
        for name in ["plugin.yaml", "format.wasm"] {
            fs::copy(package.join(name), installed.join(name))?;
        }
        Ok(installed)
    }

    fn load_rust(ardent: &Path) -> Result<FormatPlugin, Box<dyn Error>> {
        FormatPlugin::load(ardent, "rust")?.ok_or_else(|| "missing installed Rust plugin".into())
    }

    #[test]
    fn missing_directory_is_distinct_from_incomplete_package() -> TestResult {
        let ardent = tempfile::tempdir()?;
        assert!(FormatPlugin::load(ardent.path(), "rust")?.is_none());
        fs::create_dir_all(ardent.path().join("plugins/rust"))?;
        let error = load_rust(ardent.path())
            .err()
            .ok_or("accepted an empty package")?;
        assert!(error.to_string().contains("plugin.yaml"));
        Ok(())
    }

    #[test]
    fn invalid_installed_names_fail_even_when_disabled() -> TestResult {
        let ardent = tempfile::tempdir()?;
        assert!(
            InstalledPackages::discover(ardent.path())?
                .check_other_names(|_| false)
                .is_empty()
        );
        for name in ["structure", "Bad_Name"] {
            let dir = ardent.path().join("plugins").join(name);
            fs::create_dir_all(&dir)?;
            assert!(
                InstalledPackages::discover(ardent.path())?
                    .check_other_names(|_| false)
                    .is_empty()
            );
            fs::write(dir.join("plugin.yaml"), "")?;
            let errors = InstalledPackages::discover(ardent.path())?.check_other_names(|_| false);
            assert_eq!(errors.len(), 1);
            assert!(errors[0].to_string().contains(name), "{}", errors[0]);
            fs::remove_file(dir.join("plugin.yaml"))?;
        }
        Ok(())
    }

    #[test]
    fn disabled_format_and_view_names_must_match_their_directories() -> TestResult {
        let ardent = tempfile::tempdir()?;
        for (name, kind) in [("html", "format"), ("comments", "view")] {
            let dir = ardent.path().join("plugins").join(name);
            fs::create_dir_all(&dir)?;
            let manifest = dir.join("plugin.yaml");
            let write_name = |declared: &str| {
                fs::write(
                    &manifest,
                    format!("schema: 1\nname: {declared}\nkind: {kind}\n"),
                )
            };
            write_name("other")?;
            let packages = InstalledPackages::discover(ardent.path())?;
            let errors = packages.check_other_names(|_| false);
            assert_eq!(errors.len(), 1);
            let message = errors[0].to_string();
            assert!(
                message.contains("plugin.yaml")
                    && message.contains(name)
                    && message.contains("other"),
                "{message}"
            );
            write_name(name)?;
            assert!(packages.check_other_names(|_| false).is_empty());
        }
        Ok(())
    }

    #[test]
    fn enabled_format_requires_exact_directory_spelling() -> TestResult {
        let ardent = tempfile::tempdir()?;
        let dir = install_rust(ardent.path())?;
        fs::rename(&dir, dir.with_file_name("Rust"))?;
        let packages = InstalledPackages::discover(ardent.path())?;
        let errors = packages.check_other_names(|name| name == "rust");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("Rust"));
        if dir.exists() {
            let err = packages
                .load_format("rust")
                .err()
                .ok_or("accepted a case alias")?;
            assert!(err.to_string().contains("spelling"));
        } else {
            assert!(packages.load_format("rust")?.is_none());
        }
        assert!(!ardent.path().join(VERIFIED_DIR).exists());
        Ok(())
    }

    #[test]
    fn rejects_oversized_package_files_before_verification() -> TestResult {
        let ardent = tempfile::tempdir()?;
        let dir = install_rust(ardent.path())?;
        for (name, limit) in [
            ("format.wasm", MAX_COMPONENT_BYTES),
            ("plugin.yaml", MAX_MANIFEST_BYTES),
        ] {
            fs::File::options()
                .write(true)
                .open(dir.join(name))?
                .set_len(limit + 1)?;
            let error = load_rust(ardent.path())
                .err()
                .ok_or("accepted oversized file")?
                .to_string();
            assert!(
                error.contains(name) && error.contains("byte limit"),
                "{error}"
            );
        }
        assert!(!ardent.path().join(VERIFIED_DIR).exists());
        let sample = ardent.path().join("sample");
        fs::write(&sample, "abc")?;
        assert_eq!(read_file(&sample, 3)?, b"abc");
        Ok(())
    }

    #[test]
    fn invalid_manifest_fails_before_compiling_the_component() -> TestResult {
        let ardent = tempfile::tempdir()?;
        let dir = ardent.path().join("plugins/rust");
        fs::create_dir_all(&dir)?;
        let manifest = dir.join("plugin.yaml");
        let text = "schema: 1\nname: rust\nversion: 0.1.0\nkind: format\ncomponent: format.wasm\n";
        for (invalid, reason) in [
            (text.replace("schema: 1", "schema: 2"), "unknown schema 2"),
            (text.replace("name: rust", "name: other"), "other"),
            (text.replace("kind: format", "kind: view"), "kind"),
            (format!("{text}unknown: 1\n"), "unknown"),
        ] {
            fs::write(&manifest, invalid)?;
            let error = load_rust(ardent.path())
                .err()
                .ok_or("accepted invalid manifest")?
                .to_string();
            assert!(
                error.contains("plugin.yaml") && error.contains(reason),
                "{error}"
            );
        }
        Ok(())
    }

    #[test]
    fn only_verified_identical_bytes_skip_runtime_startup() -> TestResult {
        let ardent = tempfile::tempdir()?;
        let dir = install_rust(ardent.path())?;
        let first = load_rust(ardent.path())?;
        assert!(matches!(first.scanner, Scanner::Active(_)));
        let mut cached = load_rust(ardent.path())?;
        assert!(matches!(cached.scanner, Scanner::Verified(_)));

        let component = dir.join("format.wasm");
        let original = fs::read(&component)?;
        assert_eq!(first.format_hash, structure::content_hash(&original));
        fs::write(&component, b"invalid component")?;
        assert_eq!(cached.scan("test.rs", b"fn run() {}")?[0].label, "fn run");
        assert!(load_rust(ardent.path()).is_err());

        // An empty custom section changes the digest without changing the guest's version or behavior.
        let mut rebuilt = original;
        rebuilt.extend_from_slice(&[0, 1, 0]);
        fs::write(&component, &rebuilt)?;
        let updated = load_rust(ardent.path())?;
        assert!(matches!(updated.scanner, Scanner::Active(_)));
        assert_eq!(updated.version, first.version);
        assert_eq!(updated.format_hash, structure::content_hash(&rebuilt));

        let manifest = dir.join("plugin.yaml");
        fs::OpenOptions::new()
            .append(true)
            .open(&manifest)?
            .write_all(b"\n# repackaged\n")?;
        assert!(matches!(
            load_rust(ardent.path())?.scanner,
            Scanner::Active(_)
        ));
        assert!(matches!(
            load_rust(ardent.path())?.scanner,
            Scanner::Verified(_)
        ));

        let record = ardent.path().join(VERIFIED_DIR).join("rust.yaml");
        fs::write(
            &record,
            fs::read_to_string(&record)?.replace(INTERFACE, "ardent:format/scanner@9.9.9"),
        )?;
        assert!(matches!(
            load_rust(ardent.path())?.scanner,
            Scanner::Active(_)
        ));
        fs::remove_file(record)?;
        assert!(matches!(
            load_rust(ardent.path())?.scanner,
            Scanner::Active(_)
        ));
        Ok(())
    }

    #[test]
    fn mismatched_manifest_never_gets_a_verification_record() -> TestResult {
        let ardent = tempfile::tempdir()?;
        let dir = install_rust(ardent.path())?;
        let manifest = dir.join("plugin.yaml");
        let text = fs::read_to_string(&manifest)?;
        fs::write(&manifest, text.replace("*.rs", "*.txt"))?;
        let error = load_rust(ardent.path())
            .err()
            .ok_or("accepted a mismatch")?;
        assert!(error.to_string().contains("describe"), "{error}");
        assert!(!ardent.path().join("cache/verified/rust.yaml").exists());
        Ok(())
    }

    #[test]
    fn package_paths_stay_in_the_package() -> TestResult {
        let dir = tempfile::tempdir()?;
        assert!(FormatPlugin::load(dir.path(), "../rust").is_err());
        assert!(package_file(dir.path(), Path::new("../outside")).is_err());
        assert!(package_file(dir.path(), dir.path()).is_err());
        fs::create_dir(dir.path().join("wasm"))?;
        assert_eq!(
            package_file(dir.path(), Path::new("./wasm/scanner"))?,
            dir.path().join("wasm/scanner")
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn refuses_links_to_files_and_directories() -> TestResult {
        use std::os::unix::fs::symlink;

        let ardent = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let plugins = ardent.path().join("plugins");
        symlink(outside.path(), &plugins)?;
        assert!(FormatPlugin::load(ardent.path(), "rust").is_err());
        fs::remove_file(&plugins)?;
        let dir = install_rust(ardent.path())?;
        symlink(outside.path(), dir.join("linked"))?;
        assert!(package_file(&dir, Path::new("linked/scanner")).is_err());
        let manifest = dir.join("plugin.yaml");
        fs::rename(&manifest, outside.path().join("manifest"))?;
        symlink(outside.path().join("manifest"), &manifest)?;
        assert!(FormatPlugin::load(ardent.path(), "rust").is_err());
        Ok(())
    }
}
