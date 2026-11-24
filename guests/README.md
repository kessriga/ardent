# Building the format plugins

Run the commands below from the repository root. Install the Rust target `wasm32-wasip2` and WASI SDK 34. Set
`WASI_SDK_PATH` to the unpacked SDK. The C compiler in the SDK builds tree-sitter and the two grammars.

```sh
rustup target add wasm32-wasip2
export CC_wasm32_wasip2="$WASI_SDK_PATH/bin/clang"
export AR_wasm32_wasip2="$WASI_SDK_PATH/bin/llvm-ar"
cargo_registry=${CARGO_HOME:-$HOME/.cargo}
export RUSTFLAGS="--remap-path-prefix=$PWD=/ardent --remap-path-prefix=$cargo_registry=/cargo"
export RUSTFLAGS="$RUSTFLAGS --remap-path-prefix=$(rustc --print sysroot)=/rust"
export CFLAGS_wasm32_wasip2="-ffile-prefix-map=$cargo_registry=/cargo -ffile-prefix-map=$WASI_SDK_PATH=/wasi-sdk"
cargo build --locked --release --target wasm32-wasip2 --manifest-path guests/Cargo.toml --target-dir target/guests \
    -p ardent-markdown-component -p ardent-rust-component
```

Use a Python environment with `wasmtime==51.0.0` to package the components. This command reads the metadata from each
component's `describe` export and writes the installable packages under `plugins/`.

```sh
python guests/package.py target/guests/wasm32-wasip2/release plugins
```

To rebuild the component that exercises failure paths in the CLI integration tests, use the same environment.

```sh
cargo build --locked --release --target wasm32-wasip2 --manifest-path guests/Cargo.toml --target-dir target/guests \
    -p test-scanner
cp target/guests/wasm32-wasip2/release/test_scanner.wasm crates/ardent-cli/tests/components/test-scanner.wasm
```

Run the guest query and grammar tests on the native target.

```sh
cargo test --locked --workspace --manifest-path guests/Cargo.toml --target-dir target/guests
```

## Host limits for format plugins

A scanner receives source bytes and the WASI 0.2 clock, random and environment functions it needs for deterministic
execution. Clocks report the epoch or zero, and random values are repeatable. Each call can request up to 64 KiB of
random bytes. The scanner sees empty arguments and environment variables. Calls that require files, sockets or blocking
I/O trap.

The host gives instantiation and each call 5 billion units of Wasmtime fuel. It allows up to four linear memories of 64
MiB each and limits data returned across the component interface to 64 MiB per transfer. The host accepts up to 256
nested outline nodes and a conservative 64 MiB total for their expanded node paths, including the file path, escaped
labels and duplicate suffixes. Each file also has a fingerprint budget of 268,435,456 byte operations. Each byte in a
node costs one operation plus one per label range. When a scan exceeds a limit, sync reports the file and keeps its last
good structure. After an execution trap, the next file gets a fresh instance and budget.
