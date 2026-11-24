---
id: decision-8
title: Run format plugins as WebAssembly components
date: '2026-11-24 12:06'
status: accepted
---
## Context

Ardent will distribute Markdown and Rust as installable components. M1 links both grammars into the binary. TASK-11
tested whether a component can preserve the existing outline engine and how much time a sync needs to start it. Backlog
doc-2 records the release build, fixture, and cache measurements.

## Decision

The complete WebAssembly Interface Types (WIT) definition is in `spikes/task-11/wit/format.wit`. The package is
`ardent:format@0.1.0`. The world is `format-plugin`. The package version identifies the interface. The plugin version
identifies a release of a format plugin.

- `describe` returns the plugin name, version, default file patterns and default search prefixes. It can run before the
  guest compiles its query.
- `scan` accepts source bytes. Success returns nodes in document order, with each parent before its descendants. A
  top-level node has no parent. Other nodes give the zero-based index of their parent. Each node carries its kind,
  label, half-open byte range and label ranges. The host checks parent indices, tree order, containment and source
  bounds before building the outline. It adds the file root, escapes labels, assigns duplicate suffixes, and computes
  fingerprints in Ardent core.
- A syntax error returns the one-based line and character column of the first error. A plugin failure carries a message.
  Wasmtime traps, resource-limit errors and malformed results also fail the scan. Keep the last good structure and
  report the affected file or plugin. Continue syncing other files and return a failure status.
- For query-based formats, the author supplies a grammar, outline query and metadata to a shared guest-side engine. The
  build links these into one WASI 0.2 component. A format that needs custom code implements the same world. The host
  supplies source bytes, restricts memory and execution time, denies ambient file, network and environment access, and
  uses deterministic clocks and randomness. TASK-12 implements these host limits.

### Installed files and fast sync

Install `plugin.yaml` and `format.wasm` in `.ardent/plugins/<name>/` under version control. The manifest retains schema,
name, version, kind, files and prefixes, and names the component. Generate the manifest's metadata from the release
component's `describe` export when packaging. When loading it, the host checks the exported metadata against the
manifest and reports a mismatch. Project configuration overrides the default patterns and prefixes under the existing
rules. Only enabled project plugins run. The release can ship ready-to-install components separately from the Ardent
binary.

For each enabled format, hash the installed component bytes once per sync. Use the full lowercase BLAKE3 hex digest as
the existing stamp's `format_hash`, alongside format name and plugin version. Compare these before using the source
metadata shortcut. Read the manifest to select files. A digest change invalidates old structures even when the author
reuses a version. A missing or old-format stamp requires a scan. A pattern change alone updates tracking according to
the existing rules.

Defer creating Wasmtime until a tracked file actually needs a scan. Enable Wasmtime's per-user compile cache for changed
files. Create one instance per needed format per sync and build its query at the first scan. Reuse that instance for
later files. A warm compile cache saves component compilation across processes. The first-scan measurements show why the
guest still needs to reuse the query inside a process. Keep compiled native code in the runtime's trusted local cache.
The project ships portable component bytes.

### Rules carried forward

| Decision | Rule |
| --- | --- |
| 3 | Each matched syntax node yields one node. The first matching pattern wins. The query supplies kind and label captures, normalizes whitespace, and falls back to kind for an empty label. Use containment and syntax ancestry to resolve equal ranges. Ardent core owns paths and fingerprints. Keep the view plugin interface and anchor rules. |
| 5 | A positive `level` nests subsequent siblings up to the next equal or smaller level and extends the node's end accordingly. Reject invalid levels. |
| 6 | `@annotation` widens the node range and contributes to its fingerprint. `@node` retains identity, and label ranges remain separate. |
| 7 | An `ERROR` or `MISSING` yields only the first error position. Keep the previous structure and stamp, or leave the structure absent. Continue the sync and exit with status 1. |

## Consequences

This decision replaces the format-plugin and M1 built-in sections of decision-3, including the host grammar loader and
its Wasmtime version pin. It supersedes the format-plugin installation, manifest, and format hash rules of decision-4.
TASK-12 implements the host and moves Markdown and Rust out of the binary. TASK-13 makes ardent init install the release
plugins the user selects.
