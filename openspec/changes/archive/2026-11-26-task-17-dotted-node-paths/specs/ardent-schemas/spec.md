# ardent-schemas changes

## MODIFIED Requirements

### Requirement: Unknown schema in the cache

When a file in `.ardent/cache/` has no `schema` or a version that Ardent does not know, Ardent SHALL delete the file and
build it again.

#### Scenario: Structure file from another version

- **WHEN** `.ardent/cache/files/src/engine.rs/structure.yaml` has `schema: 7`
- **THEN** the next sync scans `src/engine.rs` and writes a structure file with `schema: 2`

### Requirement: Full node paths

Every node path in every file in `.ardent/` SHALL be a full node path, as the outline spec and decisions 1 and 10 define
it: the source path, `#`, and the labels from the root.

#### Scenario: One search finds every use

- **WHEN** a person searches `.ardent/` for `src/engine.rs#impl Engine/fn run`
- **THEN** the search finds the structure node, each derived entry for the node, and each stored link to the node

### Requirement: Structure nodes

A structure file SHALL list the nodes of the outline under `nodes`, in document order, root first. Each node SHALL have
`path`, `label`, `kind`, `range`, and `fingerprint`. The label SHALL contain the unescaped display text, including a
position for repeated siblings. The file node SHALL display the source path. The root node SHALL have the kind `file`
and the range of the whole file.

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
schema: 2
stamp:
  size: 201
  mtime: 2026-11-20T08:15:42.123456789Z
  hash: 3818c7c62f776addba39b4a76689ed32986e9e765a465f05d2131650b1c92203
  format: markdown
  format_version: 0.1.0
  format_hash: f44946daaae00ba28ceea81715b037f6b691458a901d5ac95a012af336cc5831
nodes:
  - path: docs/spec.md
    label: docs/spec.md
    kind: file
    range: 0..201
    fingerprint: 64d285e69608de32
  - path: docs/spec.md#paragraph.1
    label: paragraph
    kind: paragraph
    range: 0..49
    fingerprint: 986a9a5665640a09
  - path: docs/spec.md#Sync rules
    label: Sync rules
    kind: heading
    range: 50..172
    fingerprint: 6afe874c4163626f
  - path: docs/spec.md#Sync rules/paragraph.1
    label: paragraph
    kind: paragraph
    range: 65..99
    fingerprint: c066d35079ec2897
  - path: docs/spec.md#Sync rules/Renames
    label: Renames
    kind: heading
    range: 100..172
    fingerprint: f2009c466ed1d13a
  - path: docs/spec.md#Sync rules/Renames/paragraph.1
    label: paragraph
    kind: paragraph
    range: 113..171
    fingerprint: 0b019916628c0b2b
  - path: docs/spec.md#Example
    label: Example
    kind: heading
    range: 172..201
    fingerprint: 5547ffff36732ee1
  - path: docs/spec.md#Example/paragraph.1
    label: paragraph
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
schema: 2
stamp:
  size: 188
  mtime: 2026-11-20T08:15:42.123456789Z
  hash: bc0644bb4c4483bf81dfa058498edb35826be58d2bd29f040f0607b59aee46a0
  format: rust
  format_version: 0.1.0
  format_hash: 657afc1b346ceeac0ae244c175c42108bbd218969077353426ade6eb6b5f6f52
nodes:
  - path: src/engine.rs
    label: src/engine.rs
    kind: file
    range: 0..188
    fingerprint: 174add0298423e83
  - path: src/engine.rs#impl Engine
    label: impl Engine
    kind: impl
    range: 47..187
    fingerprint: e756306dadb3fb00
  - path: src/engine.rs#impl Engine/fn run
    label: fn run
    kind: fn
    range: 65..124
    fingerprint: a33b27c3382d2785
  - path: src/engine.rs#impl Engine/fn reset
    label: fn reset
    kind: fn
    range: 130..185
    fingerprint: 0c3892e426b726a6
```
