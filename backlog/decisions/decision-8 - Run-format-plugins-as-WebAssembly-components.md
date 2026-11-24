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
`ardent:format@0.1.0`. The world is `format-plugin`. It exports the named interface `ardent:format/scanner@0.1.0`. The
host resolves this export before calling `describe` or `scan`. An absent or unsupported interface is a plugin load
error. The version returned by `describe` identifies the plugin release.

- `describe` returns the plugin name, version, default file patterns and default search prefixes. It can run before the
  guest compiles its query.
- For a given component and source byte sequence, `scan` must return the same nodes, ranges, labels or syntax error on
  every supported platform, in a fresh instance and after any sequence of previous scans. A plugin may retain a compiled
  query or other state only if it preserves this result.
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
name, version, kind, files and prefixes, and names the component. Generate the manifest metadata from the release
component's `describe` export when packaging.

Before using an enabled format's defaults for discovery or pruning, verify the manifest and component together. Check
the manifest schema, kind and directory name. Compare its name, version, default patterns and prefixes with `describe`
from the supported interface. Cache successful verification as derived data, keyed by the manifest bytes, component
bytes and interface the host supports. Use the same component bytes for hashing, verification and scanning. A missing
verification record or a change to either file requires a new check, even if the manifest matches no source files. Apply
project configuration overrides after verification.

On a verification error, report the plugin and return a failure status. Treat discovery as incomplete: skip
stale-structure deletion for that sync, retain the affected structures and stamps, and continue with files that other
verified formats can scan. For example, changing only the Rust manifest from `*.rs` to `*.txt` must report a mismatch
and keep existing Rust structures, even in a project without text files.

For each enabled format, hash the installed component bytes once per sync. Use the full lowercase BLAKE3 hex digest as
the existing stamp's `format_hash`, alongside format name and plugin version. Compare these before using the source
metadata shortcut. A digest change invalidates old structures even when the author reuses a version. A missing or
old-format stamp requires a scan. A change to project pattern overrides updates tracking under the existing rules.

Start Wasmtime when a format needs verification or a file needs scanning. A sync with verified packages and current
files needs neither operation. Reuse compiled code from Wasmtime's trusted local cache. Give each active format its own
store and instance. Build the query on its first scan and reuse it after success or a returned scan error, including a
syntax error. If a call traps or exhausts its execution budget, keep the failed file's last good structure and stamp.
Discard that store and instance. Before scanning the next file of that format, instantiate the compiled component in a
fresh store with a fresh budget. Other formats keep their own instances.

### Configuration and cache scope

TASK-12 starts with project-installed format plugins. The agreed follow-up allows plugins and settings in the user's XDG
configuration as well as in the project. Project settings take precedence for overlapping keys, and a project plugin
wins over a user plugin of the same name. Wasmtime's derived compilation cache may be per-user or per-project. The cache
holds compiled code; the plugin installation holds the portable component and manifest. TASK-13 should account for both
installation scopes when deciding what initialization copies into a project.

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
