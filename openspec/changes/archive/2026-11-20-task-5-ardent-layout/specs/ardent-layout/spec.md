# Spec Delta

## Purpose

Defines where Ardent keeps each file in a project's `.ardent/` directory, what `ardent init` writes there, how the
project configuration selects plugins and tracked files, and where plugins live. The layout follows decision-4.

## ADDED Requirements

### Requirement: Project root

The project root SHALL be the nearest directory, starting at the working directory and going up, that contains a
`.ardent/` directory. Every command other than `ardent init` SHALL use that project root.

#### Scenario: Command in a subdirectory

- **WHEN** a person runs an Ardent command in `docs/` of a project whose root contains `.ardent/`
- **THEN** the command uses the project root, not `docs/`

#### Scenario: No project

- **WHEN** a person runs an Ardent command other than `ardent init`, and no ancestor directory contains `.ardent/`
- **THEN** the command fails with a message that the directory is not inside an Ardent project, and writes no file

### Requirement: Source paths

Every source path in an Ardent file SHALL be relative to the project root and SHALL use `/` between its parts, on every
platform.

#### Scenario: Path on Windows

- **WHEN** Ardent tracks the file `src\engine.rs` on Windows
- **THEN** every Ardent file writes its source path as `src/engine.rs`

### Requirement: User data and cache

`.ardent/` SHALL hold user data in `config.yaml`, `plugins/`, and `views/`, and derived data only in `cache/`. Ardent
SHALL write `.ardent/.gitignore` with the single line `/cache/`, so git ignores the cache and tracks the user data.

#### Scenario: Git status after a sync

- **WHEN** a sync has written structure files and the backlink index in a git repository
- **THEN** `git status` lists no file under `.ardent/cache/`

### Requirement: Initial content

`ardent init` SHALL create `.ardent/` with exactly two files: `.gitignore` and `config.yaml`. The configuration SHALL
enable the built-in formats with their default patterns, and no views or excludes.

#### Scenario: Files after init

- **WHEN** a person runs `ardent init` in a directory without `.ardent/`
- **THEN** `.ardent/` contains only `.gitignore` and `config.yaml`
- **AND** `.ardent/.gitignore` contains the line `/cache/`
- **AND** `.ardent/config.yaml` contains:

```yaml
schema: 1
formats:
  markdown:
    files:
      - "*.md"
      - "*.markdown"
  rust:
    files:
      - "*.rs"
views: {}
exclude: []
```

### Requirement: Directories on demand

Ardent SHALL create `plugins/`, `views/`, `cache/`, and their subdirectories only when it first writes a file into them.

#### Scenario: First structure file

- **WHEN** the first sync after `ardent init` scans `src/engine.rs`
- **THEN** Ardent creates `.ardent/cache/files/src/engine.rs/` and writes `structure.yaml` into it

### Requirement: Structure file location

The structure file of a tracked source file SHALL be `.ardent/cache/files/<source path>/structure.yaml`.

#### Scenario: Rust file

- **WHEN** a sync scans `src/engine.rs`
- **THEN** its structure file is `.ardent/cache/files/src/engine.rs/structure.yaml`

### Requirement: View file location

The stored entries of one view for one source file SHALL be in `.ardent/views/<source path>/<view>.yaml`, where `<view>`
is the name of the view plugin.

#### Scenario: Comment on a function

- **WHEN** a person adds a comment with the `comments` view to `src/engine.rs#impl Engine/fn run`
- **THEN** Ardent writes the entry into `.ardent/views/src/engine.rs/comments.yaml`

### Requirement: Derived view file location

The output of a derived view for one source file SHALL be in `.ardent/cache/files/<source path>/<view>.yaml`.

#### Scenario: Complexity of a Rust file

- **WHEN** the `complexity` view computes entries for `src/engine.rs`
- **THEN** Ardent writes them into `.ardent/cache/files/src/engine.rs/complexity.yaml`

### Requirement: Backlink index location

The backlink index SHALL be `.ardent/cache/backlinks.yaml`. Ardent SHALL derive it from the view files only, and SHALL
rebuild it when it is missing.

#### Scenario: Deleted index

- **WHEN** a person deletes `.ardent/cache/backlinks.yaml` and then runs any Ardent command
- **THEN** the command rebuilds the index from the view files, with the same content as before the deletion

### Requirement: Cache can be deleted

Ardent SHALL rebuild every file in `.ardent/cache/` from the source files and the view files when the cache is missing.

#### Scenario: Deleted cache, unchanged sources

- **WHEN** a person deletes `.ardent/cache/`, changes no source file, and runs any Ardent command
- **THEN** the command rebuilds every structure file, derived view file, and the backlink index
- **AND** no view file changes

### Requirement: Empty files and directories

Ardent SHALL delete a view file that has no entries left, and SHALL remove each directory under `.ardent/views/` and
`.ardent/cache/files/` that becomes empty.

#### Scenario: Last entry deleted

- **WHEN** a person deletes the only entry in `.ardent/views/src/engine.rs/comments.yaml`, and `src/engine.rs` has no
  other view file
- **THEN** Ardent deletes `comments.yaml` and the directory `.ardent/views/src/engine.rs/`

### Requirement: Entries follow their anchors

A stored entry SHALL live in the view file, for its view, of the source file in its anchor path. When the source file in
an anchor path changes, Ardent SHALL move the entry into the view file of the new source file at the same sync.

#### Scenario: Node moved to another file

- **WHEN** a person moves `fn run` from `src/engine.rs` into `src/runner.rs` with the same body, and a sync runs
- **THEN** the anchor path of its comment becomes `src/runner.rs#fn run`
- **AND** the entry moves from `.ardent/views/src/engine.rs/comments.yaml` to
  `.ardent/views/src/runner.rs/comments.yaml`

#### Scenario: Source file renamed

- **WHEN** a person renames `src/engine.rs` to `src/core.rs`, and a sync runs
- **THEN** every entry anchored in `src/engine.rs` moves into the view files under `.ardent/views/src/core.rs/`

#### Scenario: Anchor path edited by hand

- **WHEN** an agent changes an anchor path in `.ardent/views/src/engine.rs/comments.yaml` to a node in `docs/spec.md`
- **THEN** the next sync moves the entry into `.ardent/views/docs/spec.md/comments.yaml`

#### Scenario: Orphan stays

- **WHEN** a sync cannot resolve the anchor of an entry, and the entry becomes an orphan
- **THEN** the entry stays in its view file, with its last known path

### Requirement: Plugin names

A plugin name SHALL start with a lowercase ASCII letter and contain only lowercase ASCII letters, digits, and hyphens.
The name `structure` SHALL be reserved. Ardent SHALL reject a plugin with an invalid or reserved name.

#### Scenario: Reserved name

- **WHEN** `.ardent/plugins/structure/plugin.yaml` exists, or `config.yaml` enables a view named `structure`
- **THEN** every Ardent command fails with a message that names the reserved plugin name

### Requirement: Format plugins in the configuration

`config.yaml` SHALL enable format plugins by name under `formats`. Only the listed format plugins SHALL run. `files` and
`prefixes` under a format SHALL replace the plugin's defaults; when a key is absent, the plugin's default SHALL apply.

#### Scenario: Pattern override

- **WHEN** `config.yaml` lists `markdown` with `files: ["docs/**/*.md"]`
- **THEN** Ardent tracks `docs/spec.md` and does not track `README.md`

#### Scenario: Format not listed

- **WHEN** `config.yaml` lists only `markdown` under `formats`
- **THEN** Ardent tracks no `.rs` file

#### Scenario: Prefix override

- **WHEN** `config.yaml` lists `markdown` with `prefixes: {spec: ["docs/**/*.md"]}`
- **THEN** the search prefix `spec` covers only Markdown files under `docs/`

### Requirement: View plugins in the configuration

`config.yaml` SHALL enable view plugins by name under `views`, as a mapping from the plugin name to its settings. Only
the listed view plugins SHALL run.

#### Scenario: View enabled

- **WHEN** `config.yaml` contains `views: {comments: {}}`
- **THEN** Ardent runs the `comments` view plugin and no other view plugin

### Requirement: Patterns

Every pattern in `config.yaml` and in a plugin manifest SHALL use gitignore syntax, relative to the project root. A
pattern without `/` SHALL match at any depth.

#### Scenario: Pattern without a slash

- **WHEN** a format has the pattern `*.md`
- **THEN** it matches `README.md` and `docs/guide/intro.md`

### Requirement: Tracked files

Ardent SHALL track a file when the `files` patterns of exactly one enabled format match it, git's ignore rules do not
ignore it, no `exclude` pattern matches it, it is outside `.ardent/`, and its path is valid UTF-8.

#### Scenario: Ignored by git

- **WHEN** `.gitignore` contains `target/` and `target/doc/notes.md` exists
- **THEN** Ardent does not track `target/doc/notes.md`

#### Scenario: Excluded

- **WHEN** `config.yaml` contains `exclude: ["vendor/"]`
- **THEN** Ardent does not track any file under `vendor/`

#### Scenario: Two formats match

- **WHEN** two enabled formats both match `notes.md`
- **THEN** the sync reports `notes.md` and the names of both formats
- **AND** Ardent does not track `notes.md`

#### Scenario: Path is not UTF-8

- **WHEN** a file matches a format pattern and its path is not valid UTF-8
- **THEN** the sync reports the file, and Ardent does not track it

### Requirement: Plugin locations

Ardent SHALL load plugins only from its binary (the built-in plugins) and from `.ardent/plugins/<name>/`. A project
plugin SHALL replace the built-in plugin with the same name.

#### Scenario: Pinned Markdown plugin

- **WHEN** `.ardent/plugins/markdown/plugin.yaml` declares `version: 0.0.9`, and a sync scans `README.md`
- **THEN** the stamp of the structure of `README.md` has `format: markdown` and `format_version: 0.0.9`

### Requirement: Plugin manifest

Each project plugin SHALL have a manifest `plugin.yaml` with `schema`, `name`, `version`, and `kind`. `name` SHALL equal
the name of its directory. `kind` SHALL be `format` or `view`. File names in the manifest SHALL be relative to the
plugin directory.

#### Scenario: Format manifest

- **WHEN** `.ardent/plugins/html/plugin.yaml` contains:

```yaml
schema: 1
name: html
version: 0.3.1
kind: format
grammar: grammar.wasm
outline: outline.scm
files:
  - "*.html"
prefixes:
  page:
    - "*.html"
```

- **THEN** Ardent loads the grammar from `.ardent/plugins/html/grammar.wasm` and the outline query from
  `.ardent/plugins/html/outline.scm`

#### Scenario: View manifest

- **WHEN** `.ardent/plugins/comments/plugin.yaml` contains:

```yaml
schema: 1
name: comments
version: 0.1.0
kind: view
component: comments.wasm
```

- **THEN** Ardent loads the view plugin from `.ardent/plugins/comments/comments.wasm`

#### Scenario: Name does not match the directory

- **WHEN** `.ardent/plugins/html/plugin.yaml` contains `name: xml`
- **THEN** every Ardent command fails with a message that names the manifest and both names

### Requirement: Lock

During a sync and the writes after it, Ardent SHALL hold an exclusive operating-system lock on `.ardent/cache/lock`. A
second Ardent process SHALL wait for the lock. The lock SHALL end when the process that holds it ends.

#### Scenario: TUI and CLI at the same time

- **WHEN** the TUI is in a sync and an agent runs an Ardent command
- **THEN** the command starts its own sync only after the TUI's sync has written its results

#### Scenario: Crash during a sync

- **WHEN** an Ardent process ends during a sync without releasing the lock
- **THEN** the next Ardent command gets the lock without waiting and without manual cleanup

### Requirement: Atomic writes

Ardent SHALL write each file in `.ardent/` into `.ardent/cache/tmp/` first and then rename it into place.

#### Scenario: Crash during a write

- **WHEN** an Ardent process ends while it writes a view file
- **THEN** the view file has either its old content or its new content
- **AND** any partly written file is under `.ardent/cache/tmp/`
