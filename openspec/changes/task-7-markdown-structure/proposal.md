# Proposal

## Why

`ardent init` creates a project, but no command reads a source file yet. Markdown is the simplest structured format, so
it proves the document model end to end: from a source file, through a format plugin, to a structure file in
`.ardent/cache/`. TASK-8 (Rust) and TASK-9 (TUI) build on the outline types and the sync that this change adds. This
change implements Backlog TASK-7.

## What Changes

- Add the command `ardent sync`. It lists the tracked files, re-scans each file whose stamp changed, writes its
  structure file, and deletes the structure files of files that are no longer tracked. It holds the lock on
  `.ardent/cache/lock` while it works.
- Add the outline engine. It runs the outline query of a format plugin on a tree-sitter parse tree and gives the nodes
  of the outline with their kinds, labels, byte ranges, and label ranges.
- Add the optional query property `level`. A node with a level contains the siblings after it up to the next sibling
  with the same or a lower level. ATX and setext headings in Markdown nest through this property. A new Backlog
  decision, decision-5, records this addition to the query parts of decision-3.
- Add the built-in `markdown` format plugin, version `0.1.0`. Its nodes are headings, paragraphs, lists, list items,
  code blocks, block quotes, tables, and front matter.
- Add the outline and structure types to `ardent-core`: node paths with escapes and `[n]` suffixes (decision-1),
  fingerprints and stamps (spec `ardent-schemas`), and reading and writing of structure files.
- Read `config.yaml` with its schema version, and report unknown keys.
- Skip each directory below the project root that holds its own `.ardent/` directory, and report it as a nested project.
- Fix the problems that the review of PR #7 found:
  - `write_atomic` writes only to relative paths inside `.ardent/`, and it creates the parent directories of the target.
  - The project root search reports I/O errors and does not treat them as "no project here".

Out of scope:

- The matcher, link re-resolution, view files, and the backlink index. M1 has no views, so a sync has no links to
  re-resolve.
- The Rust format plugin (TASK-8). Until TASK-8, `ardent sync` reports that the `rust` format in the initial
  configuration has no plugin.
- Project plugins in `.ardent/plugins/`, plugin manifests, and grammars loaded from `.wasm` files.
- Search prefixes. Ardent reads `prefixes` from `config.yaml` but does not use them yet.
- HTML blocks, thematic breaks, and link reference definitions as nodes. Their text stays in the range of the parent
  node.
- Options for `ardent sync`, a file watcher, and the TUI (TASK-9).

## Capabilities

### New Capabilities

- `outline`: how Ardent builds the outline of a source file from the outline query of a format plugin: query parts,
  labels, nesting by containment and by level, node paths, and positional suffixes.
- `markdown-format`: the built-in Markdown format plugin: its name, version, default file patterns, node kinds, labels,
  and how headings nest.
- `sync`: the `ardent sync` command: which files it scans, how the stamp check skips unchanged files, which structure
  files it deletes, and what it reports.

### Modified Capabilities

- `ardent-layout`: "Project root" reports I/O errors and uses the working directory with symlinks resolved. "Tracked
  files" excludes the files of nested projects.

## Impact

- New crate `ardent-format` with the outline engine and the built-in Markdown plugin. The dependency direction becomes
  `ardent-core ← ardent-format ← ardent-cli` (decision-3).
- `ardent-core` gains the modules `outline`, `structure`, `config`, and `yaml`, and `layout` gains path checks.
- `ardent-cli` gains the `sync` command and the modules `tracked` and `sync`. `project_root::find` returns
  `io::Result<Option<PathBuf>>`, and `ardent init` handles the error.
- New dependencies:
  - `tree-sitter` 0.27.2 and `tree-sitter-md` 0.5.4: the parser basis of decision-3.
  - `serde` and `serde-saphyr` 2.2: YAML. `serde_yaml` is unmaintained. `serde-saphyr` writes the format of the spec
    examples byte for byte and names the line of an unknown key.
  - `blake3`: fingerprints and hashes (decision-4).
  - `jiff`: the `mtime` of a stamp as RFC 3339 with nanoseconds. `humantime` panics on times before 1970.
  - `ignore`: the file walk with git's ignore rules, and patterns in gitignore syntax.
  - `insta`, only for development: snapshot tests.
