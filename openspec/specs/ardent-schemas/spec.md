# ardent-schemas Specification

## Purpose

Defines the YAML schema of each kind of file in `.ardent/`: the schema version, the rules that every file follows, the
fingerprint, structure files, view files, derived view files, and the backlink index. The schemas follow decision-4 and
use the node paths, links, and statuses of decision-1.

## Requirements

### Requirement: Schema version

Every YAML file that Ardent reads or writes in `.ardent/` SHALL start with the key `schema`, whose value is an integer.
Version 1 SHALL be the first version of every kind of file. The `schema` of `config.yaml` SHALL also version the layout
of `.ardent/`.

#### Scenario: New view file

- **WHEN** Ardent writes a new view file
- **THEN** its first line is `schema: 1`

### Requirement: Unknown schema in user data

When `config.yaml`, a plugin manifest, or a view file has no `schema` or a version that Ardent does not know, Ardent
SHALL report the file and SHALL NOT change it. For `config.yaml` or a manifest, the command SHALL fail. For a view file,
the sync SHALL go on with the other files.

#### Scenario: View file from a newer Ardent

- **WHEN** `.ardent/views/src/engine.rs/comments.yaml` has `schema: 2`, and an Ardent that knows only version 1 syncs
- **THEN** the sync reports the file and its version
- **AND** the file keeps its content byte for byte

#### Scenario: Configuration without a version

- **WHEN** `.ardent/config.yaml` has no `schema` key
- **THEN** every Ardent command fails with a message that names the file

### Requirement: Unknown schema in the cache

When a file in `.ardent/cache/` has no `schema` or a version that Ardent does not know, Ardent SHALL delete the file and
build it again.

#### Scenario: Structure file from another version

- **WHEN** `.ardent/cache/files/src/engine.rs/structure.yaml` has `schema: 7`
- **THEN** the next sync scans `src/engine.rs` and writes a structure file with `schema: 1`

### Requirement: Key style and order

Ardent SHALL write block-style YAML with snake_case keys, in the order that this spec lists them for each kind of file.

#### Scenario: Order of entry keys

- **WHEN** Ardent writes a stored entry that has every key
- **THEN** the keys come in the order `id`, `anchor`, `fields`, `refs`

### Requirement: Ranges

Ardent SHALL write each byte range and line range as a string `start..end`, where `start` is included and `end` is not.

#### Scenario: Byte range of a function

- **WHEN** a function starts at byte 65 and its last byte is byte 123
- **THEN** its structure node has `range: 65..124`

### Requirement: Full node paths

Every node path in every file in `.ardent/` SHALL be a full node path, as decision-1 defines it: the source path, `#`,
and the labels from the root.

#### Scenario: One search finds every use

- **WHEN** a person searches `.ardent/` for `src/engine.rs#impl Engine/fn run`
- **THEN** the search finds the structure node, each derived entry for the node, and each stored link to the node

### Requirement: Optional keys

Ardent SHALL leave out an optional key that has no value, and a list that is empty.

#### Scenario: Entry without references

- **WHEN** an entry's text values contain no reference
- **THEN** the entry has no `refs` key

### Requirement: Unknown keys

Ardent SHALL reject a key that this spec does not define, except inside `fields`. It SHALL report the file and the key,
and handle the file as a file with an unknown schema.

#### Scenario: Typo in an anchor

- **WHEN** an agent writes `anchr:` in place of `anchor:` in a view file
- **THEN** the sync reports the file and the key `anchr`
- **AND** the file keeps its content byte for byte

### Requirement: Fingerprint

A fingerprint SHALL be the BLAKE3 hash of the node's normalized text, cut to its first 8 bytes, as 16 lowercase hex
digits. To normalize, Ardent SHALL take the bytes of the node's range, remove its label ranges, replace each run of
ASCII whitespace (bytes 0x09 to 0x0D and 0x20) with one space, and remove a space at the start or the end. The root node
SHALL use the whole file and no label ranges.

#### Scenario: Function in the Rust example

- **WHEN** Ardent computes the fingerprint of `src/engine.rs#impl Engine/fn run` in the Rust example, whose label ranges
  are `69..71` (`fn`) and `72..75` (`run`)
- **THEN** the normalized text is `pub (&self) -> usize { self.files.len() }`
- **AND** the fingerprint is `a33b27c3382d2785`

#### Scenario: Line endings

- **WHEN** a checkout has CRLF line endings in `src/engine.rs` in place of LF
- **THEN** every node of `src/engine.rs` has the same fingerprint as in the LF checkout

#### Scenario: Rename

- **WHEN** a person renames `fn run` to `fn start` and changes nothing else
- **THEN** the fingerprint of the renamed node is still `a33b27c3382d2785`

### Requirement: Stamp

A structure file SHALL have a `stamp` with these keys: `size` in bytes; `mtime`, the modification time as an RFC 3339
UTC timestamp at the precision the file system gives; `hash`, the full BLAKE3 hash of the file as 64 lowercase hex
digits; and `format`, `format_version`, and `format_hash` of the format plugin.

#### Scenario: Stamp of the Rust example

- **WHEN** a sync scans the 188 bytes of the Rust example
- **THEN** the stamp has `size: 188` and `hash: bc0644bb4c4483bf81dfa058498edb35826be58d2bd29f040f0607b59aee46a0`

### Requirement: Format hash

For every format plugin, `format_hash` SHALL be the BLAKE3 hash of the complete `format.wasm` file, as 64 lowercase hex
digits.

#### Scenario: Changed query

- **WHEN** a person installs a rebuilt `.ardent/plugins/html/format.wasm` with a changed query and the same manifest
  version
- **THEN** the next sync finds a different `format_hash` and scans every tracked HTML file again

### Requirement: Structure nodes

A structure file SHALL list the nodes of the outline under `nodes`, in document order, root first. Each node SHALL have
`path`, `kind`, `range`, and `fingerprint`. The root node SHALL have the kind `file` and the range of the whole file.

#### Scenario: Markdown example

- **WHEN** a format plugin with the Markdown example query of decision-3 scans `docs/spec.md` with this content:

```markdown
Ardent derives an outline from each source file.

## Sync rules

Every command starts with a sync.

### Renames

A rename keeps the fingerprint.
It also keeps every link.

## Example

A short example.
```

- **THEN** `.ardent/cache/files/docs/spec.md/structure.yaml` contains, with example values for `mtime` and
  `format_hash`:

```yaml
schema: 1
stamp:
  size: 201
  mtime: 2026-11-20T08:15:42.123456789Z
  hash: 3818c7c62f776addba39b4a76689ed32986e9e765a465f05d2131650b1c92203
  format: markdown
  format_version: 0.1.0
  format_hash: f44946daaae00ba28ceea81715b037f6b691458a901d5ac95a012af336cc5831
nodes:
  - path: docs/spec.md
    kind: file
    range: 0..201
    fingerprint: 64d285e69608de32
  - path: docs/spec.md#paragraph
    kind: paragraph
    range: 0..49
    fingerprint: 986a9a5665640a09
  - path: docs/spec.md#Sync rules
    kind: heading
    range: 50..172
    fingerprint: 6afe874c4163626f
  - path: docs/spec.md#Sync rules/paragraph
    kind: paragraph
    range: 65..99
    fingerprint: c066d35079ec2897
  - path: docs/spec.md#Sync rules/Renames
    kind: heading
    range: 100..172
    fingerprint: f2009c466ed1d13a
  - path: docs/spec.md#Sync rules/Renames/paragraph
    kind: paragraph
    range: 113..171
    fingerprint: 0b019916628c0b2b
  - path: docs/spec.md#Example
    kind: heading
    range: 172..201
    fingerprint: 5547ffff36732ee1
  - path: docs/spec.md#Example/paragraph
    kind: paragraph
    range: 184..201
    fingerprint: 7b8f8a6fa531fab0
```

#### Scenario: Rust example

- **WHEN** a format plugin with the Rust example query of decision-3 scans `src/engine.rs` with this content:

```rust
pub struct Engine {
    files: Vec<String>,
}

impl Engine {
    pub fn run(&self) -> usize {
        self.files.len()
    }

    fn reset(&mut self) {
        self.files.clear();
    }
}
```

- **THEN** `.ardent/cache/files/src/engine.rs/structure.yaml` contains, with example values for `mtime` and
  `format_hash`:

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

### Requirement: View file

A view file SHALL hold `schema` and `entries`, a list of stored entries. Each entry SHALL have `id`, `anchor`, and
`fields`, and SHALL have `refs` when its text values contain references. Ardent SHALL write `id`, `anchor`, and `refs`;
the view plugin SHALL own `fields`.

#### Scenario: Rust example

- **WHEN** a person comments on `fn run` of the Rust example, and `fn reset` was named `fn clear` before the last sync
- **THEN** `.ardent/views/src/engine.rs/comments.yaml` contains:

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

#### Scenario: Markdown example

- **WHEN** a person comments on one line of `Renames` in the Markdown example, and an older comment points at a
  `Glossary` section that no longer exists
- **THEN** `.ardent/views/docs/spec.md/comments.yaml` contains:

```yaml
schema: 1
entries:
  - id: 0f6yq3ncs1
    anchor:
      path: docs/spec.md#Sync rules/Renames
      fingerprint: f2009c466ed1d13a
      status: confirmed
      quote: It also keeps every link.
      lines: 3..4
    fields:
      text: Only links in stored views. The code is in @[src/engine.rs#impl Engine/fn run].
      resolved: false
    refs:
      - path: src/engine.rs#impl Engine/fn run
        fingerprint: a33b27c3382d2785
        status: confirmed
  - id: m3tz8e5kdp
    anchor:
      path: docs/spec.md#Glossary
      fingerprint: 1c0d7e6a9b2f4053
      status: orphan
    fields:
      text: Define the stamp here.
      tags:
        - glossary
        - todo
```

### Requirement: Fields

`fields` SHALL be a mapping from field name to value. A YAML string SHALL be a text, a YAML number a number, a YAML
boolean a flag, and a list of YAML strings a list of texts. Ardent SHALL keep the fields in the order that the view
plugin returns them.

#### Scenario: Field types in the Markdown example

- **WHEN** Ardent reads the entry `m3tz8e5kdp` of the Markdown example
- **THEN** it passes `text` to the view plugin as a text and `tags` as a list of two texts

### Requirement: Anchor

An anchor SHALL have `path`, `fingerprint`, and `status`. It SHALL have `was` only when its status is `unconfirmed`. It
MAY narrow to part of the node with `quote` and `lines`, where `lines` holds line offsets from the first line of the
node. It SHALL have `lines` only together with `quote`.

#### Scenario: Quote in the Markdown example

- **WHEN** Ardent resolves the anchor of entry `0f6yq3ncs1` in the Markdown example
- **THEN** it finds `It also keeps every link.` on line offset 3 of `docs/spec.md#Sync rules/Renames`

#### Scenario: Confirmation

- **WHEN** a person confirms the reference to `src/engine.rs#impl Engine/fn reset` in the Rust example
- **THEN** its status becomes `confirmed` and its `was` key disappears

### Requirement: References list

`refs` SHALL have one item for each distinct node path in the entry's text values, in the order of first appearance:
fields in their order, and the texts of a list in their order. Each item SHALL have `path`, `fingerprint`, and `status`,
and `was` under the same rule as an anchor.

#### Scenario: Two references

- **WHEN** the text of an entry is `See @[b.md#B], @[a.md#A], and @[b.md#B] again.`
- **THEN** `refs` has two items, first `b.md#B` and then `a.md#A`

### Requirement: Link status

The status of an anchor or a `refs` item SHALL be `confirmed`, `unconfirmed`, or `orphan`, with the meaning of
decision-1. An orphan SHALL keep its last known path and fingerprint.

#### Scenario: Orphan in the Markdown example

- **WHEN** no node matches the anchor of entry `m3tz8e5kdp`
- **THEN** its anchor keeps `path: docs/spec.md#Glossary` and `fingerprint: 1c0d7e6a9b2f4053`, with `status: orphan`

### Requirement: Entry ID

Each stored entry SHALL have an `id` of 10 random characters from `0123456789abcdefghjkmnpqrstvwxyz` (lowercase
Crockford base32). An ID SHALL be unique in the project.

#### Scenario: New entry

- **WHEN** a person adds a comment
- **THEN** its `id` matches `^[0-9a-hjkmnp-tv-z]{10}$` and no other entry in the project has it

### Requirement: Entry order

Ardent SHALL insert each entry that it creates, moves, or gives an ID before the first entry in the file whose ID sorts
after its own ID, in byte order. Ardent SHALL NOT reorder the existing entries of a file.

#### Scenario: Insert by ID

- **WHEN** a view file holds entries with IDs `0f6yq3ncs1` and `m3tz8e5kdp`, and Ardent adds an entry with ID
  `9xw2m4bt7c`
- **THEN** the new entry is the second entry of the file

#### Scenario: Path rewrite

- **WHEN** a sync rewrites the anchor path of an entry inside the same source file
- **THEN** the entry keeps its place in the file

#### Scenario: Concurrent additions merge

- **WHEN** two git branches each add one entry to the same view file, and the new IDs sort into different places
- **THEN** git merges the two branches without a conflict

### Requirement: Entry without ID

When a view file has an entry without `id`, the next sync SHALL give it a new ID and move it to its place by that ID.

#### Scenario: Entry written by an agent

- **WHEN** an agent appends an entry without `id` at the end of a view file
- **THEN** the next sync gives the entry an ID and moves it to its place by that ID

### Requirement: Duplicate IDs

When two entries have the same ID, the sync SHALL give a new ID to one of them and report the change. It SHALL change
the entry in the view file whose path sorts later, or the later entry when both are in one file.

#### Scenario: Same ID after a merge

- **WHEN** `views/docs/spec.md/comments.yaml` and `views/src/engine.rs/comments.yaml` both hold an entry with ID
  `4k2m9qx7tb`
- **THEN** the sync gives a new ID to the entry in `views/src/engine.rs/comments.yaml` and reports both files

### Requirement: Derived view file

A derived view file SHALL hold `schema`, `plugin_version`, `source_hash` (the `hash` of the source file it was computed
from), and `entries`. Each entry SHALL have `node`, a full node path, and `fields`. Ardent SHALL compute the file again
when the plugin version or the source hash differs.

#### Scenario: Complexity of the Rust example

- **WHEN** a `complexity` view plugin, version 0.2.0, computes entries for the Rust example
- **THEN** `.ardent/cache/files/src/engine.rs/complexity.yaml` contains:

```yaml
schema: 1
plugin_version: 0.2.0
source_hash: bc0644bb4c4483bf81dfa058498edb35826be58d2bd29f040f0607b59aee46a0
entries:
  - node: src/engine.rs#impl Engine/fn run
    fields:
      complexity: 1
  - node: src/engine.rs#impl Engine/fn reset
    fields:
      complexity: 1
```

#### Scenario: Plugin upgrade

- **WHEN** the `complexity` plugin changes to version 0.3.0
- **THEN** the next sync computes `complexity.yaml` again for every tracked file

### Requirement: Backlink index

The backlink index SHALL hold `schema`, `view_files`, and `targets`. `view_files` SHALL map the path of each view file,
relative to `.ardent/`, to its `size`, `mtime`, and `hash`. `targets` SHALL map each node path, in byte order, to the
links that point at it: `entry`, `file`, `view`, and `link` (`anchor` or `ref`).

#### Scenario: Index of both examples

- **WHEN** the project holds the Markdown and Rust view files of this spec
- **THEN** `.ardent/cache/backlinks.yaml` contains, with example values for `mtime`:

```yaml
schema: 1
view_files:
  views/docs/spec.md/comments.yaml:
    size: 673
    mtime: 2026-11-20T08:20:11.402913000Z
    hash: 20e9981b677355d5935f18dd1f283ba58f82f71fb84405936a37c5cede23ecf7
  views/src/engine.rs/comments.yaml:
    size: 578
    mtime: 2026-11-20T08:19:54.118204000Z
    hash: abeb18e77e1d73937cecc46448f4057f2074d18e048531f264a6bba17ab7463d
targets:
  docs/spec.md#Glossary:
    - entry: m3tz8e5kdp
      file: docs/spec.md
      view: comments
      link: anchor
  docs/spec.md#Sync rules:
    - entry: 4k2m9qx7tb
      file: src/engine.rs
      view: comments
      link: ref
  docs/spec.md#Sync rules/Renames:
    - entry: 0f6yq3ncs1
      file: docs/spec.md
      view: comments
      link: anchor
  src/engine.rs#impl Engine/fn reset:
    - entry: 4k2m9qx7tb
      file: src/engine.rs
      view: comments
      link: ref
  src/engine.rs#impl Engine/fn run:
    - entry: 0f6yq3ncs1
      file: docs/spec.md
      view: comments
      link: ref
    - entry: 4k2m9qx7tb
      file: src/engine.rs
      view: comments
      link: anchor
```

### Requirement: Order of index rows

Within a target, the backlink index SHALL list links in the byte order of their view file paths, then in the order of
the entries in the file, with an anchor before a reference of the same entry.

#### Scenario: Two links to fn run

- **WHEN** an entry in `views/docs/spec.md/comments.yaml` references `fn run`, and an entry in
  `views/src/engine.rs/comments.yaml` is anchored to it
- **THEN** the row from `views/docs/spec.md/comments.yaml` comes first

### Requirement: Stored entries only

The backlink index SHALL cover the links of stored entries only. It SHALL NOT list references in derived entries.

#### Scenario: Reference in a derived entry

- **WHEN** a derived view writes `@[docs/spec.md#Example]` into a text field
- **THEN** the backlink index has no row for that reference
