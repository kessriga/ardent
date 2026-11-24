# Proposal

## Why

Today, updating a Markdown or Rust grammar requires replacing the Ardent binary. TASK-12 lets a project pin its own
format plugins. Decision-8 defines how the host loads them and how a sync handles a failed scan.

## What Changes

- **BREAKING** Projects must install the Markdown and Rust components they enable. The CLI will ship without embedded
  grammars or plugins. Until TASK-13, people copy each package into `.ardent/plugins/<name>/`.
- Keep the existing node paths, ranges, fingerprints and syntax error positions. Verify installed metadata before using
  it to discover files. Preserve the last good structure on scan failure.
- Run each active format in an isolated Wasmtime instance. A sync with verified packages and current files can finish
  without starting Wasmtime.

## Capabilities

### New Capabilities

- `format-components` defines the scanner interface, limits on guest execution, and checks on returned nodes.

### Modified Capabilities

- `ardent-layout` defines where sync finds packages and which formats a new project enables.
- `ardent-schemas` uses the component digest for all format stamps.
- `markdown-format` and `rust-format` describe the two release components and their identities.
- `sync` covers the first sync with installed packages, package verification, and migration from native plugin stamps.

## Impact

The change affects the format engine, the CLI sync path, release packaging, test fixtures and user documentation. Ardent
core still owns node paths and fingerprints. Plugin selection during init (TASK-13), user-wide settings and cache
selection (TASK-14), and view plugins are outside this task.
