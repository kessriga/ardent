# Spec Delta

## MODIFIED Requirements

### Requirement: Key style and order

Ardent SHALL write block-style YAML with snake_case keys, in the order that this spec lists them for each kind of file.

#### Scenario: Order of entry keys

- **WHEN** Ardent writes a stored entry that has every key
- **THEN** the keys come in the order `id`, `created`, `anchor`, `fields`, `refs`

### Requirement: View file

A view file SHALL hold `schema` and `entries`, a list of stored entries. Each entry SHALL have `id`, `created`,
`anchor`, and `fields`, and SHALL have `refs` when its text values contain references. Ardent SHALL write `id`,
`created`, `anchor`, and `refs`; the view plugin SHALL own `fields`.

#### Scenario: Rust example

- **WHEN** a person comments on `fn run` of the Rust example, and `fn reset` was named `fn clear` before the last sync
- **THEN** `.ardent/views/src/engine.rs/comments.yaml` contains:

```yaml
schema: 1
entries:
  - id: 4k2m9qx7tb
    created: 2026-11-20T08:20:00Z
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
    created: 2026-11-20T08:16:00Z
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
    created: 2026-11-20T08:10:00Z
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
    size: 741
    mtime: 2026-11-20T08:20:11.402913000Z
    hash: 3c7a87b4fa6317512a35c477b49f9b837a915bc310af6465691dda0c5e749fb2
  views/src/engine.rs/comments.yaml:
    size: 612
    mtime: 2026-11-20T08:20:04.118204000Z
    hash: 34b2e2bf78ed427ac276953622d4839ee9a7fc4eaeed3ea8d4c04cae350a3877
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

## ADDED Requirements

### Requirement: Stored view entry creation time

When Ardent creates a stored view entry, it SHALL write the current host time in `created` as an RFC 3339 UTC timestamp
with a `Z` suffix. When sync first sees an otherwise valid stored entry without `created`, it SHALL fill in the current
host time. Ardent SHALL preserve a valid `created` across edits, syncs, ID repairs and moves between view files. For a
malformed `created`, it SHALL report the affected file and key, preserve the file and continue with other files.

#### Scenario: Entry written outside Ardent

- **WHEN** a sync at `2026-11-20T08:20:00Z` first sees a valid comment without an ID or creation time
- **THEN** it writes a new ID and `created: 2026-11-20T08:20:00Z` and places the entry in ID order

#### Scenario: Comment follows its node

- **WHEN** sync moves a comment to another view file after its node moves to another source file
- **THEN** the comment keeps its original creation time

#### Scenario: Invalid time

- **WHEN** an external writer gives an entry `created: yesterday`
- **THEN** sync reports its view file and `created`, preserves the file byte for byte and continues with other files
