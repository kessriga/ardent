# rust-format changes

## MODIFIED Requirements

### Requirement: Item labels

The label of an item node SHALL be its keyword and its name, such as `fn run`, `struct Engine`, `type Output`, or
`macro_rules! log`. Visibility, qualifiers such as `async`, `unsafe`, or `mut`, generic parameters, and the rest of the
item SHALL NOT be part of the label.

#### Scenario: Public async function with generics

- **WHEN** `src/lib.rs` has the item `pub async fn fetch<T: Send>(url: &str) -> T`
- **THEN** its node path is `src/lib.rs#fn fetch`

#### Scenario: Two functions with one name

- **WHEN** `src/lib.rs` has `#[cfg(unix)] fn home()` and then `#[cfg(windows)] fn home()`
- **THEN** their node paths are `src/lib.rs#fn home.1` and `src/lib.rs#fn home.2`

### Requirement: Fields and variants

Each field of a struct, a union, or an enum variant SHALL be a node of the kind `field`, with the field name as its
label. A field of a tuple struct or a tuple variant SHALL have the label `field`. Each enum variant SHALL be a node of
the kind `variant`, with the variant name as its label.

#### Scenario: Named field

- **WHEN** `src/engine.rs` has `pub struct Engine { files: Vec<String> }`
- **THEN** the field has the node path `src/engine.rs#struct Engine/files` and the kind `field`

#### Scenario: Enum with a struct variant and a tuple variant

- **WHEN** `src/entry.rs` has `enum Entry { File, Dir { path: PathBuf }, Link(PathBuf, bool) }`
- **THEN** the outline has the nodes `src/entry.rs#enum Entry/File`, `src/entry.rs#enum Entry/Dir/path`,
  `src/entry.rs#enum Entry/Link/field.1`, and `src/entry.rs#enum Entry/Link/field.2`

### Requirement: Doc comments and attributes

The outer doc comments (`///` and `/** */`) and outer attributes (`#[...]`) directly before an item, a field, or an enum
variant SHALL be part of its node, and the range of the node SHALL start at the first of them. Blank lines between them
SHALL NOT end the run, and an ordinary comment SHALL end it. Inner doc comments (`//!`) and inner attributes (`#![...]`)
SHALL stay part of the node that contains them.

#### Scenario: Documented struct

- **WHEN** `src/engine.rs` starts with the lines `/// The engine.`, `#[derive(Debug)]`, and `pub struct Engine;`
- **THEN** the node `src/engine.rs#struct Engine` has the range `0..51`

#### Scenario: Edited doc comment

- **WHEN** a person changes the doc comment of `fn run` and changes nothing else
- **THEN** the fingerprint of the node `fn run` changes

#### Scenario: Ordinary comment between a doc comment and its item

- **WHEN** `src/lib.rs` has the lines `/// Runs it.`, `// TODO: faster`, and `fn run() {}`
- **THEN** the range of the node `src/lib.rs#fn run` starts at `fn run`

#### Scenario: Attribute on a tuple field

- **WHEN** `src/id.rs` has `struct Id(#[serde(skip)] pub u64);`
- **THEN** the range of the node `src/id.rs#struct Id/field.1` starts at `#[serde(skip)]`
