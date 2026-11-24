---
id: decision-4
title: >-
  The .ardent/ directory keeps views per source file in git and derived data in
  a rebuildable cache
date: '2026-11-20 08:26'
status: accepted
---
## Context

Decision-8 supersedes the format-plugin installation, manifest, and format hash rules in this record. It also assigns
plugin selection during initialization to TASK-13.

decision-1, decision-2, and decision-3 left the on-disk form of `.ardent/` to TASK-5. They ask this record to:

- split `.ardent/` into a part in git and a derived part, with one ignore rule for the derived part (decision-2);
- map each source file to its structure file and view files, and name the backlink index and the lock (decision-2);
- fix the YAML spellings of anchors, `refs`, statuses, and `was`, and pick the fingerprint hash (decision-1);
- decide whether a view entry moves when its node moves to another file (decision-1);
- keep view entries in a stable order, so that git merges stay small (decision-2);
- fix where plugins live, the manifest spellings, how the project configuration enables plugins and overrides file
  patterns, where plugin fields go in an entry, and where derived views live (decision-3).

Four choices came from the design discussion:

- Ardent groups view files per source file: one directory for each source file, one file for each view.
- A view entry always lives in the view file of its anchor's source file. When a node moves to another file, its entries
  move with it.
- Ardent, not the view plugin, decides which file an entry goes in and where in that file. Entries have random IDs and
  stay in ID order. The view plugin decides what the fields mean and how the TUI orders entries.
- The similarity phase of the matcher needs the old text of leaf nodes (decision-2), and the source file no longer has
  it after an outside edit. The task that builds the matcher decides how the cache keeps that text.

The OpenSpec specs `ardent-layout` and `ardent-schemas` hold the full schema, with worked examples for a Markdown file
and a Rust file. This record gives the choices and the reasons for them.

## Decision

### Directory layout

The project root is the directory that contains `.ardent/`. A command that runs in a subdirectory uses the nearest
ancestor directory that contains `.ardent/`. Every source path in an Ardent file is relative to the project root and
uses `/` between its parts.

```text
.ardent/
  .gitignore                        one line: /cache/
  config.yaml                       project configuration
  plugins/<plugin>/plugin.yaml      project plugins, with their files
  views/<source path>/<view>.yaml   stored views
  cache/                            derived part
    lock
    backlinks.yaml
    tmp/
    files/<source path>/structure.yaml
    files/<source path>/<view>.yaml derived views
```

| Part      | Paths                               | In git |
| --------- | ----------------------------------- | ------ |
| User data | `config.yaml`, `plugins/`, `views/` | Yes    |
| Derived   | `cache/`                            | No     |

- Ardent appends the source path to `views/` or `cache/files/` as directories. The source file `src/engine.rs` has its
  comments in `.ardent/views/src/engine.rs/comments.yaml` and its structure in
  `.ardent/cache/files/src/engine.rs/structure.yaml`. A path in the source tree is a file or a directory, never both, so
  the directory for one source file never clashes with the directory for another.
- The name of a view file is the name of its view plugin. A plugin name starts with a lowercase letter and contains only
  lowercase letters, digits, and hyphens. The name `structure` is reserved, because `structure.yaml` and the derived
  view files share a directory.
- Ardent never tracks files inside `.ardent/`.
- Ardent deletes a view file that has no entries, and removes empty directories under `views/` and `cache/files/`.
- `ardent init` writes `config.yaml` and `.gitignore`. Ardent creates the other directories when it first writes into
  them.
- Ardent can delete `cache/` at any time, and a full sync rebuilds it. The previous outlines are lost, with the effect
  of a fresh clone (decision-2).

### Schema versions

Every YAML file that Ardent reads or writes starts with `schema: <integer>`. Version 1 is the first version of every
kind of file. The `schema` of `config.yaml` also versions the directory layout.

- **User data** (configuration, plugin manifests, view files). Ardent reads only versions that it knows. When the
  version is missing or unknown, Ardent reports the file and does not change it. A later change that raises a version
  must say how Ardent migrates older files.
- **Cache files.** When the version is missing or unknown, Ardent deletes the file and rebuilds it. A change to a cache
  file therefore needs no migration.

The key is `schema` and not `version`, because `version` in a plugin manifest is the version of the plugin.

### Rules for every file

- Keys use snake_case. Ardent writes block-style YAML, with keys in the order that the spec lists them.
- A byte range or line range is a string `start..end`, half-open as in Rust: `65..124`. It fits on one line.
- Every node path is a full node path (decision-1), in cache files too. One `grep` for a path finds the structure node,
  the derived entries, and the stored entries that use it.
- Ardent leaves out optional keys and empty lists.
- Ardent rejects keys that the schema does not define, and reports the file. A typo by a person or an agent therefore
  fails loudly and does not drop data. Plugin fields (see "View files") are free.

### Project configuration

`.ardent/config.yaml` decides which plugins run and which files Ardent tracks:

```yaml
schema: 1
formats:
  markdown:
    files:
      - "*.md"
      - "*.markdown"
    prefixes:
      spec:
        - docs/**/*.md
  rust:
    files:
      - "*.rs"
views:
  comments: {}
exclude:
  - vendor/
```

- `formats` enables format plugins by name. Only listed formats run. `files` and `prefixes` replace the defaults of the
  plugin; when a key is absent, the plugin default applies.
- `views` enables view plugins by name. It is a mapping, like `formats`, so that a later version can add settings for a
  view without a new shape for this user file.
- `exclude` removes files from tracking.
- Patterns use gitignore syntax and are relative to the project root. A pattern without `/` matches at any depth.

A file is tracked when all of these are true:

1. The `files` patterns of exactly one enabled format match it.
2. Git's ignore rules (`.gitignore` files and `.git/info/exclude`) do not ignore it.
3. No `exclude` pattern matches it.
4. It is not inside `.ardent/`.
5. Its path is valid UTF-8.

A sync reports each file that two formats match and each file with a path that is not UTF-8, and tracks neither.

Ardent follows git's ignore rules because views go into git. A view of a file that git ignores would point at a file
that other clones do not have.

`ardent init` writes the two built-in formats with their default patterns, `views: {}`, and `exclude: []`. The explicit
patterns show a person which files Ardent tracks.

### Plugins

- Built-in plugins ship inside the binary. In M1 these are `markdown` and `rust`.
- Project plugins live in `.ardent/plugins/<name>/`, which is in git. A project plugin replaces a built-in plugin with
  the same name, so a project can pin the version of a plugin.
- There is no per-user plugin directory. Labels, node paths, and fingerprints depend on the format plugin. If two
  machines had different plugins, the same file would give different node paths on each machine, and every sync would
  rewrite links in view files that other people share.

A manifest is `plugin.yaml` in the plugin directory:

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

```yaml
schema: 1
name: comments
version: 0.1.0
kind: view
component: comments.wasm
```

- `name` equals the name of the plugin directory. `version` is a semantic version. `kind` is `format` or `view`. File
  names are relative to the plugin directory.
- The format hash in the stamp (decision-3) is the BLAKE3 hash of the grammar file followed by the query file. For a
  built-in plugin, it is the hash of the query text followed by the name and version of the grammar crate.

### Fingerprints and hashes

A fingerprint is the BLAKE3 hash of the normalized text of a node, cut to its first 8 bytes and written as 16 lowercase
hex digits. Ardent normalizes the text of a node in three steps:

1. Take the bytes of the node's range, and remove the label ranges (decision-1).
2. Replace each run of ASCII whitespace (tab, line feed, vertical tab, form feed, carriage return, space) with one
   space.
3. Remove the space at the start and at the end.

The root node of a file uses the whole file and has no label ranges.

- The whitespace rule uses ASCII, not Unicode, so it never depends on a Unicode version. A checkout with CRLF line
  endings gives the same fingerprints as one with LF.
- BLAKE3 has a fixed specification, gives the same output on every platform, and has a maintained Rust crate, `blake3`.
- With 64 bits, the chance that two of one million nodes share a fingerprint is about 3 in 100 million.
- The content hash in a stamp is the full BLAKE3 hash, 64 hex digits.

### Structure files

`.ardent/cache/files/<source path>/structure.yaml` holds the stamp (decision-2) and the nodes of the outline in document
order, root first. Each node has a `path`, a `kind`, a `range`, and a `fingerprint`. The path gives the parent. The root
has the kind `file`.

```yaml
schema: 1
stamp:
  size: 188
  mtime: 2026-11-20T08:15:42.123456789Z
  hash: bc0644bb4c4483bf81dfa058498edb35826be58d2bd29f040f0607b59aee46a0
  format: rust
  format_version: 0.1.0
  format_hash: 657afc1b346ceeac0ae244c175c42108bbd218969077353426ade6eb6b5f6f52
nodes:
  - path: src/engine.rs
    kind: file
    range: 0..188
    fingerprint: 174add0298423e83
  - path: src/engine.rs#impl Engine
    kind: impl
    range: 47..187
    fingerprint: e756306dadb3fb00
  - path: src/engine.rs#impl Engine/fn run
    kind: fn
    range: 65..124
    fingerprint: a33b27c3382d2785
  - path: src/engine.rs#impl Engine/fn reset
    kind: fn
    range: 130..185
    fingerprint: 0c3892e426b726a6
```

The structure does not store label ranges. A scan needs them only to compute fingerprints. The later feature that edits
source files through the outline adds them; since the structure is a cache file, that change needs no migration.

### View files

`.ardent/views/<source path>/<view>.yaml` holds the stored entries of one view for one source file:

```yaml
schema: 1
entries:
  - id: 4k2m9qx7tb
    anchor:
      path: src/engine.rs#impl Engine/fn run
      fingerprint: a33b27c3382d2785
      status: confirmed
    fields:
      text: This function contradicts @[docs/spec.md#Sync rules]. See also @[src/engine.rs#impl Engine/fn reset].
      resolved: false
    refs:
      - path: docs/spec.md#Sync rules
        fingerprint: 6afe874c4163626f
        status: confirmed
      - path: src/engine.rs#impl Engine/fn reset
        fingerprint: 0c3892e426b726a6
        status: unconfirmed
        was: src/engine.rs#impl Engine/fn clear
```

- An entry has `id`, `anchor`, `fields`, and `refs`. Ardent writes `id`, `anchor`, and `refs`. The view plugin owns
  `fields` (decision-3).
- `fields` is a separate mapping, so a field name never clashes with a key of Ardent, and Ardent can add keys later. A
  YAML string is a text, a number is a number, a boolean is a flag, and a list of strings is a list of texts.
- `anchor` has `path`, `fingerprint`, and `status`. It has `was` only when the status is `unconfirmed`. It can narrow to
  part of the node with `quote` and `lines`, as decision-1 describes. `lines` holds half-open line offsets from the
  first line of the node. decision-1 called it `range`; the new name keeps it apart from byte ranges. An anchor has
  `lines` only together with `quote`.
- `refs` has one item for each distinct node path in the text values of the entry, in the order of first appearance. An
  item has `path`, `fingerprint`, `status`, and `was` under the same rule as an anchor.
- The status values are `confirmed`, `unconfirmed`, and `orphan` (decision-1). An orphan keeps its last known path and
  fingerprint.

#### Entry IDs and order

- An entry ID is 10 random characters from the lowercase Crockford base32 alphabet, `0123456789abcdefghjkmnpqrstvwxyz`.
  That gives 50 bits. An ID is unique in the project.
- Ardent inserts each entry that it creates, moves, or gives an ID before the first entry whose ID sorts after it.
  Ardent never reorders the entries of a file.
- An entry without `id`, such as one that an agent wrote, gets an ID at the next sync. Ardent then moves it to its place
  by that ID.
- When a merge leaves two entries with the same ID, the sync gives a new ID to one of them, and reports the change. It
  changes the entry in the view file whose path sorts later, or the later entry when both are in one file.

Two branches that each append an entry at the end of the same file always conflict in git. Two branches that insert in
different places merge cleanly. A throwaway check with git 2.49.0 confirmed both cases. Random IDs spread new entries
over the file, so most concurrent additions merge cleanly. This replaces the rule "append new entries" of decision-2.
The aim of that rule, small merges, stays.

Ardent owns the order because a view plugin has no file access (decision-3), and because a sync places entries without
calling the plugin, for example when it moves an entry or gives it an ID. One rule for every view also gives people and
agents one file shape to learn. The display order in the TUI is the concern of the view plugin. Version `0.1.0` of the
plugin interface has no call for it; a later version can add one.

#### Entries follow their anchors

An entry lives in the view file of the source file in its anchor path. When a sync rewrites an anchor path into another
source file, Ardent moves the entry to the view file of the same view for that source file. A hand edit that changes the
file in an anchor path moves the entry at the next sync. An orphan keeps its last known path, so it stays where it is.

A renamed source file therefore takes its views with it, and the view files of a source file show every entry that is
anchored in it.

### Derived views

A derived view stores its output in `.ardent/cache/files/<source path>/<view>.yaml`. The file has `schema`,
`plugin_version`, `source_hash`, and `entries`. Each entry has `node`, a full node path, and `fields`. Ardent computes
the file again when the plugin version or the source hash differs. The task that builds the plugin host can add keys,
because the file is a cache file.

### Backlink index

The backlink index is `.ardent/cache/backlinks.yaml`. Ardent derives it from the view files. It rebuilds the index when
the index is missing or has an unknown schema, and it updates the index for each view file that changed. The index
holds:

- `view_files`: the stamp (size, modification time, hash) of each view file, by its path relative to `.ardent/`. A sync
  compares these stamps to find view files that changed outside Ardent (decision-2).
- `targets`: for each node path, the links that point at it. Each link gives the entry ID, the source file and view of
  the view file, and `link: anchor` or `link: ref`.

The index lists anchors as well as references, because a re-scan finds the links into a changed file through the index
(decision-1). The backlinks of a node are its `ref` rows. The index covers stored entries only (decision-3).

### Lock and temporary files

- `.ardent/cache/lock` is an empty file. During a sync and its writes, Ardent holds an exclusive operating-system lock
  on it, with `File::lock` from the Rust standard library. The operating system releases the lock when the process ends,
  so a crash leaves no stale lock.
- Ardent writes each file into `.ardent/cache/tmp/` and then renames it into place. A crash leaves temporary files only
  in the derived part. A rename is atomic only within one file system, so all of `.ardent/` must be on one file system.

### Left to other tasks

- How the cache keeps the old text of leaf nodes for the matcher: the task that builds the matcher.
- Label ranges in the structure: the task that adds edits through the outline.
- How to migrate a user file: the first change that raises its schema version.

## Consequences

- One ignore rule keeps the cache out of git. Ardent can delete the cache at any time; only rename tracking since the
  last sync is lost.
- The `views/` tree has the same shape as the source tree, so a person finds the views of a file at once. A deep source
  tree gives an equally deep `.ardent/` tree, and paths in `cache/files/` are about 35 characters longer than the source
  path. On Windows, a very deep source tree can reach the 260-character path limit.
- After a source file moves, the next sync moves its view files. Git shows each view file as deleted and added, and its
  rename detection usually pairs them.
- The entries of a view file are not in time order. A view plugin that needs time order stores a date field.
- Unknown keys fail loudly. Agents that write view files must use the exact spellings of the spec.
- Project plugins put `.wasm` files into the repository of the project.
- A person who wants a view plugin only for themselves must still add it to the project.
- TASK-6 writes the initial `config.yaml` and `.gitignore`. TASK-7 and TASK-8 write structure files in this schema. The
  task that builds the plugin host implements manifests, derived view files, and the backlink index.
