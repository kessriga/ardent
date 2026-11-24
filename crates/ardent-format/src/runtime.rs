//! A typed component instance belongs to one format and survives successful scans and guest-reported errors.

use std::collections::BTreeMap;

use ardent_core::outline::ScannedNode;
use thiserror::Error;
use wasmtime::component::{Component, Linker, TypedFunc};
use wasmtime::{Cache, CacheConfig, Config, Engine, Store, StoreLimits, StoreLimitsBuilder};

mod nodes;
mod wasi;

wasmtime::component::bindgen!({ path: "../../wit", world: "format-plugin" });
use exports::ardent::format::scanner::{self, ScanError};

pub(crate) const INTERFACE: &str = "ardent:format/scanner@0.1.0";
const DIAGNOSTIC_BYTES: usize = 1024;
const MEMORY_BYTES: usize = 64 << 20;
const CALL_FUEL: u64 = 5_000_000_000;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("component runtime: {0:#}")]
    Runtime(wasmtime::Error),
    #[error("syntax error at line {line}, column {column}")]
    Syntax { line: u32, column: u32 },
    #[error("component scan failed: {0}")]
    Failure(String),
    #[error("invalid component result: {0}")]
    Invalid(&'static str),
}

impl From<wasmtime::Error> for RuntimeError {
    fn from(error: wasmtime::Error) -> Self {
        Self::Runtime(error)
    }
}

fn failure_message(message: &str) -> String {
    let mut display = String::with_capacity(message.len().min(DIAGNOSTIC_BYTES));
    for ch in message.chars() {
        match ch {
            '"' | '\'' | '\\' => display.push(ch),
            _ => display.extend(ch.escape_debug()),
        }
        if display.len() > DIAGNOSTIC_BYTES {
            display.truncate(display.floor_char_boundary(DIAGNOSTIC_BYTES - 3));
            display.push_str("...");
            break;
        }
    }
    display
}

pub(crate) struct Metadata {
    pub name: String,
    pub version: String,
    pub files: Vec<String>,
    pub prefixes: BTreeMap<String, Vec<String>>,
}

pub(crate) struct Runtime {
    compiled: FormatPluginPre<StoreLimits>,
    active: Option<Instance>,
}

struct Instance {
    store: Store<StoreLimits>,
    bindings: FormatPlugin,
    prepare_query: Option<TypedFunc<(), ()>>,
}

impl Runtime {
    pub fn compile(bytes: &[u8]) -> Result<Self, RuntimeError> {
        let mut config = Config::new();
        config
            .consume_fuel(true)
            .relaxed_simd_deterministic(true)
            .cranelift_nan_canonicalization(true);
        // A read-only home can still run portable components without a compilation cache.
        config.cache(Cache::new(CacheConfig::new()).ok());
        let engine = Engine::new(&config)?;
        let component = Component::from_binary(&engine, bytes)?;
        if !component
            .component_type()
            .exports(&engine)
            .any(|(name, _)| name == INTERFACE)
        {
            return Err(RuntimeError::Invalid(
                "missing ardent:format/scanner@0.1.0 export",
            ));
        }
        let mut linker = Linker::new(&engine);
        wasi::link(&mut linker, &component)?;
        let compiled = FormatPluginPre::new(linker.instantiate_pre(&component)?)?;
        Ok(Self {
            compiled,
            active: None,
        })
    }

    fn call<T>(
        &mut self,
        run: impl FnOnce(&mut Instance) -> wasmtime::Result<T>,
    ) -> Result<T, RuntimeError> {
        let mut instance = match self.active.take() {
            Some(instance) => instance,
            None => Instance::new(&self.compiled)?,
        };
        let value = run(&mut instance)?;
        // A trap poisons the instance. Only a completed call puts it back into service.
        self.active = Some(instance);
        Ok(value)
    }

    pub fn describe(&mut self) -> Result<Metadata, RuntimeError> {
        let metadata = self.call(|i| {
            i.store.set_fuel(CALL_FUEL)?;
            i.bindings
                .ardent_format_scanner()
                .call_describe(&mut i.store)
        })?;
        let mut prefixes = BTreeMap::new();
        for prefix in metadata.prefixes {
            if prefixes.insert(prefix.name, prefix.files).is_some() {
                return Err(RuntimeError::Invalid("duplicate search prefix"));
            }
        }
        Ok(Metadata {
            name: metadata.name,
            version: metadata.version,
            files: metadata.files,
            prefixes,
        })
    }

    pub fn scan(&mut self, path: &str, source: &[u8]) -> Result<Vec<ScannedNode>, RuntimeError> {
        self.scan_with_fuel(path, source, CALL_FUEL)
    }

    fn scan_with_fuel(
        &mut self,
        path: &str,
        source: &[u8],
        fuel: u64,
    ) -> Result<Vec<ScannedNode>, RuntimeError> {
        let result = self.call(|i| {
            if let Some(prepare) = i.prepare_query.take() {
                i.store.set_fuel(CALL_FUEL)?;
                prepare.call(&mut i.store, ())?;
            }
            i.store.set_fuel(fuel)?;
            i.bindings
                .ardent_format_scanner()
                .call_scan(&mut i.store, source)
        })?;
        match result {
            Ok(nodes) => nodes::tree(nodes, path, source.len()),
            Err(ScanError::Failure(message)) => {
                Err(RuntimeError::Failure(failure_message(&message)))
            }
            Err(ScanError::Syntax(
                scanner::Position { line: 0, .. } | scanner::Position { column: 0, .. },
            )) => Err(RuntimeError::Invalid("syntax positions count from one")),
            Err(ScanError::Syntax(scanner::Position { line, column })) => {
                Err(RuntimeError::Syntax { line, column })
            }
        }
    }
}

impl Instance {
    fn new(compiled: &FormatPluginPre<StoreLimits>) -> wasmtime::Result<Self> {
        let limits = StoreLimitsBuilder::new()
            .memory_size(MEMORY_BYTES)
            .memories(4)
            .instances(16)
            .tables(16)
            .table_elements(65_536)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(compiled.engine(), limits);
        store.limiter(|limits| limits);
        store.set_hostcall_fuel(MEMORY_BYTES);
        store.set_fuel(CALL_FUEL)?;
        let instance = compiled.instance_pre().instantiate(&mut store)?;
        let bindings = FormatPlugin::new(&mut store, &instance)?;
        let prepare_query = instance
            .get_export_index(&mut store, None, "ardent:format/query-preparation@0.1.0")
            .map(|interface| {
                let prepare = instance
                    .get_export_index(&mut store, Some(&interface), "prepare")
                    .ok_or_else(|| wasmtime::Error::msg("query-preparation is missing prepare"))?;
                instance.get_typed_func(&mut store, prepare)
            })
            .transpose()?;
        Ok(Self {
            store,
            bindings,
            prepare_query,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{error::Error, fs, path::Path};

    type TestResult = Result<(), Box<dyn Error>>;

    fn probe_bytes() -> std::io::Result<Vec<u8>> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../ardent-cli/tests/components/test-scanner.wasm"),
        )
    }

    #[test]
    fn bounds_execution_and_recreates_the_store_after_a_trap() -> TestResult {
        let mut runtime = Runtime::compile(&probe_bytes()?)?;
        runtime.describe()?;
        assert!(runtime.scan_with_fuel("file", b"file", 0).is_err());
        assert!(runtime.active.is_none());
        assert_eq!(runtime.scan("file", b"next")?.len(), 1);
        let error = runtime
            .scan("file", b"memory")
            .expect_err("growth exceeds the guest memory cap");
        assert!(error.to_string().contains("grow"), "{error}");
        assert!(runtime.active.is_none());
        assert_eq!(runtime.scan("file", b"next")?.len(), 1);
        Ok(())
    }

    #[test]
    fn guest_failures_fit_one_line_and_preserve_unicode() {
        let quoted_path = r#"can't parse "C:\src\main.rs""#;
        assert_eq!(failure_message(quoted_path), quoted_path);
        assert_eq!(failure_message("é\r\n\u{1b}é"), r"é\r\n\u{1b}é");
        let fits = "é".repeat(DIAGNOSTIC_BYTES / 2);
        assert_eq!(failure_message(&fits), fits);
        let truncated = failure_message(&format!("{fits}é"));
        assert_eq!(
            truncated,
            format!("{}...", "é".repeat((DIAGNOSTIC_BYTES - 3) / 2))
        );
    }

    #[test]
    fn wasi_only_supplies_deterministic_process_inputs() -> TestResult {
        let bytes = probe_bytes()?;
        let mut runtime = Runtime::compile(&bytes)?;
        assert!(runtime.compiled.engine().get_relaxed_simd_deterministic());
        let first = runtime.scan("file", b"deterministic")?;
        assert!(
            first[0]
                .label
                .starts_with("0 arguments, 0 variables, 0 ns, hash "),
            "{first:?}"
        );
        assert_eq!(
            Runtime::compile(&bytes)?.scan("file", b"deterministic")?,
            first
        );
        for input in [b"filesystem".as_slice(), b"socket"] {
            assert!(runtime.scan("file", input).is_err());
            assert_eq!(runtime.scan("file", b"next")?.len(), 1);
        }
        Ok(())
    }

    #[test]
    fn requires_the_versioned_scanner_export() -> TestResult {
        let mut bytes = probe_bytes()?;
        let mut replaced = 0;
        for offset in 0..=bytes.len() - INTERFACE.len() {
            if bytes[offset..].starts_with(INTERFACE.as_bytes()) {
                bytes[offset + INTERFACE.len() - 1] = b'1';
                replaced += 1;
            }
        }
        assert!(replaced > 0);
        assert!(matches!(
            Runtime::compile(&bytes),
            Err(RuntimeError::Invalid(_))
        ));
        Ok(())
    }

    #[test]
    fn cold_and_warm_query_scans_get_the_same_file_budget() -> TestResult {
        let packages = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins");
        for (name, source) in [("rust", "fn sample() {}"), ("markdown", "# Sample\n")] {
            let bytes = fs::read(packages.join(name).join("format.wasm"))?;
            let mut runtime = Runtime::compile(&bytes)?;
            runtime.describe()?;
            let preparation_pending = runtime
                .active
                .as_ref()
                .is_some_and(|i| i.prepare_query.is_some());
            assert!(
                preparation_pending,
                "{name} must defer query preparation until its first scan"
            );
            let scan = |r: &mut Runtime| r.scan_with_fuel("sample", source.as_bytes(), 1_000_000);
            let first = scan(&mut runtime)?;
            assert_eq!(first.len(), 1);
            assert_eq!(scan(&mut runtime)?, first);
            assert!(
                runtime
                    .scan_with_fuel("sample", source.as_bytes(), 0)
                    .is_err()
            );
            assert_eq!(scan(&mut runtime)?, first);
        }
        Ok(())
    }

    #[test]
    fn syntax_errors_preserve_a_usable_instance() -> TestResult {
        let bytes =
            fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/rust/format.wasm"))?;
        let mut rust = Runtime::compile(&bytes)?;
        let first = rust.scan("file", b"fn valid() {}")?;
        assert!(matches!(
            rust.scan("file", b"fn invalid("),
            Err(RuntimeError::Syntax { .. })
        ));
        assert!(rust.active.is_some());
        assert_eq!(rust.scan("file", b"fn valid() {}")?, first);
        Ok(())
    }
}
