---
id: doc-2
title: Rust format component measurements
type: other
created_date: '2026-11-24 12:01'
updated_date: '2026-11-24 12:07'
---
## What ran

The release component in `spikes/task-11/` links tree-sitter 0.27.2, tree-sitter-rust 0.24.2, and the existing Rust
outline query and engine. The host passes source bytes to the guest. It reads the 721-byte `modules.rs` fixture from the
existing snapshot suite and receives 11 nodes. The self-check verifies the annotated range of `fn run` and the parent of
its nested `fn count`. The separate broken source from the native test returns line 6, column 5. Production migration
and full snapshot parity belong to TASK-12.

### Release timings

Seven fresh host processes per condition on arm64 macOS 27.2.1, with rustc 1.96.0, WASI SDK 34, CPython 3.14.8, and
Wasmtime 51.0.0. The component was 2,791,965 bytes. Each empty-cache run used a new directory. Each warm run reused the
preceding empty run's cache in a fresh process. The table gives medians in milliseconds.

| Compile cache | Load and describe | First scan | Repeated scan |
| --- | ---: | ---: | ---: |
| Disabled | 187.57 | 72.06 | 1.50 |
| Empty | 212.16 | 72.70 | 1.55 |
| Warm | 17.45 | 71.05 | 1.47 |

Load covers engine creation, linking WASI, reading and compiling the component or reading cached code, instantiation,
and metadata retrieval. First scan includes building the query inside the guest. Each process then scanned the same file
ten times with that instance. The last column summarizes the per-process medians. Scan times include the Python
binding's conversion of inputs and results. The timer excludes Python startup and fixture I/O. These figures describe
one small file and one machine.

### Reproducing a sample

Run from the repository root with the WASI SDK unpacked at `WASI_SDK_PATH`. Use a Python environment with
`wasmtime==51.0.0`. The experiment used a standalone CPython interpreter on macOS.

```sh
rustup target add wasm32-wasip2
export CC_wasm32_wasip2="$WASI_SDK_PATH/bin/clang"
export AR_wasm32_wasip2="$WASI_SDK_PATH/bin/llvm-ar"
export CARGO_TARGET_DIR="$PWD/target/task-11"
cargo build --locked --release --target wasm32-wasip2 --manifest-path spikes/task-11/Cargo.toml
guest="$CARGO_TARGET_DIR/wasm32-wasip2/release/rust_format_spike.wasm"
python spikes/task-11/measure.py "$guest"
cache_dir=$(mktemp -d)
printf '[cache]\ndirectory = "%s/compiled"\n' "$cache_dir" > "$cache_dir/cache.toml"
python spikes/task-11/measure.py "$guest" --cache-config "$cache_dir/cache.toml"
python spikes/task-11/measure.py "$guest" --cache-config "$cache_dir/cache.toml"
```

The three invocations measure disabled, empty, and warm caches in that order. The host leaves the cache disabled unless
supplied with a configuration. See the
[Rust target documentation](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip2.html) and
[Wasmtime cache documentation](https://docs.wasmtime.dev/cli-cache.html).

Decision-8 records the approved format plugin interface, installation layout and stamp rule.
