# Building the format plugins

Run the commands below from the repository root. The release components use Rust 1.96.0 and WASI SDK 34. Set
`WASI_SDK_PATH` to the unpacked SDK. Its C compiler builds tree-sitter and the two grammars.

```sh
rustup toolchain install 1.96.0 --target wasm32-wasip2
python3 guests/build.py
```

For a byte-for-byte rebuild, use the same checkout, registry and toolchain paths. Cargo includes paths in crate hashes,
so builds in different directories can produce different component bytes.

Use a Python environment with `wasmtime==51.0.0` to package the components. This command reads the metadata from each
component's `describe` export and writes the installable packages under `plugins/`. The Rust package includes the
parser-limit documentation from `guests/rust/README.md`.

```sh
python guests/package.py target/guests/wasm32-wasip2/release plugins
```

The build also produces the scanner for the CLI integration tests. Copy it to the fixture directory after rebuilding.

```sh
cp target/guests/wasm32-wasip2/release/test_scanner.wasm crates/ardent-cli/tests/components/test-scanner.wasm
```

Run the guest query and grammar tests on the native target.

```sh
cargo test --locked --workspace --manifest-path guests/Cargo.toml --target-dir target/guests
```

## Selecting the scanner world

The WIT package defines two worlds. A scanner that generates bindings with `wit-bindgen` must select `format-plugin`. If
your crate previously called `wit_bindgen::generate!()`, keep the WIT file in its `wit/` directory and select the
scanner world explicitly.

```rust
wit_bindgen::generate!({ world: "format-plugin" });
```

A query-based scanner can also export `query-setup` for separate query preparation. See
[`guests/rust/src/lib.rs`](rust/src/lib.rs) for an implementation using the shared guest bindings.

## Host limits for format plugins

A scanner receives source bytes and the WASI 0.2 clock, random and environment functions it needs for deterministic
execution. Clocks report the epoch or zero, and random values are repeatable. Each call can request up to 64 KiB of
random bytes. The scanner sees empty arguments and environment variables. Calls that require files, sockets or blocking
I/O trap.

The host accepts manifests up to 1 MiB and components up to 64 MiB. A scan accepts at most 64 MiB of source. If a source
exceeds that limit, sync keeps its last good structure and continues with the next file. The host reports the cause of
each trap, escapes control characters in scanner diagnostics and limits the diagnostic text to 1,024 UTF-8 bytes.

The host gives instantiation and each call 5 billion units of Wasmtime fuel. The packaged parsers export the optional
`ardent:format/query-preparation@0.1.0` interface. The host calls its `prepare` function once before the first scan in
each instance. Query compilation gets its own fuel budget, so every file gets the full scan budget.

The host allows up to four linear memories of 64 MiB each and limits data returned across the component interface to 64
MiB per transfer. The host accepts up to 256 nested outline nodes and a conservative 64 MiB total for their expanded
node paths, including the file path, escaped labels and duplicate suffixes. Each file also has a fingerprint budget of
268,435,456 byte operations. Each byte in a node costs one operation plus one per label range. When a scan exceeds a
limit, sync reports the file and keeps its last good structure. After an execution trap, the next file gets a fresh
instance and budget.
