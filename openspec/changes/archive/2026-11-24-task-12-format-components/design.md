# Design

## Context

Today `ardent-cli` constructs two native plugins before it discovers files. Each plugin supplies the file patterns and
the scanner. Sync prunes stale structures after discovery and before scanning. A package that fails to load can
therefore affect files well beyond the first scan. Decisions 7 and 8 define when to keep existing structures.

## Goals and non-goals

Keep the native dependency tree for `ardent-cli` free of grammars and isolate guest failures while retaining the
existing outline and structure formats. The init command keeps writing its current configuration. Users install the two
release packages manually during this milestone.

## Decisions

### Ship two packages built from the existing queries

Use one copy of the versioned WebAssembly Interface Types (WIT) contract from TASK-11 for both host and guests. Move the
existing tree-sitter engine into a shared guest crate. The Markdown and Rust guest crates each link their grammar and
outline query and export the same scanner interface. Keep query compilation lazy and reuse the query across scans.
Generate each release package's `plugin.yaml` from the built `format.wasm` by calling `describe`. Keep the spike
reproducible when its source paths move.

### Preserve the package identity throughout sync

Put package loading, verification and the Wasmtime host in `ardent-format`. Read enabled packages from their project
directories. Keep reads inside that directory and reject symlinks on the package path. Hash the component once and use
those same bytes for verification and execution. A missing package directory produces a warning. An installed but
unreadable, incomplete or inconsistent package produces an error.

Key the verification record by manifest bytes, component digest and supported interface. A missing or invalid record
requires verification even if no files match. Write derived verification records atomically under the project cache.
Validate project patterns before creating cache files to preserve the invalid-configuration contract. Keep Wasmtime's
trusted compiled-code cache separate from portable packages and project-supplied files.

### Check results at the boundary

Give each active format a separate Wasmtime store and instance, a memory limit and a fresh execution budget per call,
including `describe`. Restrict WASI imports to the capabilities in decision-8. Validate parent indices, document order,
containment and ranges before converting a result into `ardent_core::outline::ScannedNode`. Ardent core continues to
build node paths and fingerprints. A returned syntax error leaves the instance usable. A trap discards the instance and
store while retaining the compiled component for the next file.

### Use the existing sync and pruning rules

In `ardent-cli`, collect package errors before walking the project and pass only verified formats to the existing
pattern matcher. Apply the project's pattern overrides after verification. If verification or the walk fails, skip
pruning for the whole run. If a scan fails, report the file and plugin, preserve the last successful stamp, and
continue. Compare the installed component digest before the source metadata shortcut. Reuse verified metadata without
starting Wasmtime when all structures are current.

## Risks and trade-offs

- A valid Rust 2024 declaration can fail under tree-sitter-rust 0.24.2. Document the exact declaration in the Rust
  package and test its diagnostic through sync.
- Creating a fresh runtime for every file would repeat query initialization. Reuse each healthy instance and test
  recovery by scanning a valid file after an earlier trap in the same format.
- Updating snapshots during migration could hide an outline change. Run the existing expectations through the packaged
  components, including Unicode error positions and equal or nested ranges. Inspect the CLI dependency tree separately.
- Moving guest code changes the build prerequisites. Provide a repeatable package build using the Rust WASI target and
  WASI SDK for the grammar sources, and make the package fixtures available to ordinary host tests.

## Migration

Install the release packages under the existing format names before the first sync with the updated binary. That sync
replaces native-format stamps as it scans each file successfully. Source files, config and views need no conversion.
Restoring the previous binary lets its native plugins rebuild the derived structures with their original stamps.
