# rust-format Specification

## Purpose

Defines the packaged Rust format plugin: its name, version, and default file patterns, which items and members of a Rust
file become nodes of the outline, their kinds and labels, and which doc comments and attributes belong to a node.

## Requirements

### Requirement: Plugin identity

The packaged Rust format component SHALL have the name `rust` and the version `0.1.0`. Its default file pattern SHALL be
`*.rs`.

#### Scenario: Stamp of a Rust file

- **WHEN** a sync scans `src/engine.rs` with the installed Rust component
- **THEN** the stamp of its structure file has `format: rust` and `format_version: 0.1.0`

### Requirement: Item nodes

Each of these items SHALL be a node of the kind in parentheses: module (`mod`), impl block (`impl`), trait (`trait`),
function or function signature (`fn`), struct (`struct`), enum (`enum`), union (`union`), constant (`const`), static
(`static`), type alias or associated type (`type`), `macro_rules!` macro (`macro`), and extern block (`extern`). An item
at any depth SHALL be a node, also an item inside a function body.

#### Scenario: Structure of the spec example

- **WHEN** a sync scans the Rust example of the spec `ardent-schemas`, under "Structure nodes"
- **THEN** the outline has the nodes `src/engine.rs#struct Engine` and `src/engine.rs#struct Engine/files`
- **AND** the nodes `src/engine.rs#impl Engine`, `src/engine.rs#impl Engine/fn run`, and
  `src/engine.rs#impl Engine/fn reset` have the kinds, ranges, and fingerprints of the example

#### Scenario: Declared module

- **WHEN** `src/lib.rs` has the line `mod engine;`
- **THEN** the outline has the node `src/lib.rs#mod engine` with the kind `mod`

#### Scenario: Function inside a function body

- **WHEN** the function `fn run` in `src/lib.rs` defines a function `fn step` in its body
- **THEN** `fn step` has the node path `src/lib.rs#fn run/fn step`

### Requirement: Item labels

The label of an item node SHALL be its keyword and its name, such as `fn run`, `struct Engine`, `type Output`, or
`macro_rules! log`. Visibility, qualifiers such as `async`, `unsafe`, or `mut`, generic parameters, and the rest of the
item SHALL NOT be part of the label.

#### Scenario: Public async function with generics

- **WHEN** `src/lib.rs` has the item `pub async fn fetch<T: Send>(url: &str) -> T`
- **THEN** its node path is `src/lib.rs#fn fetch`

#### Scenario: Two functions with one name

- **WHEN** `src/lib.rs` has `#[cfg(unix)] fn home()` and then `#[cfg(windows)] fn home()`
- **THEN** their node paths are `src/lib.rs#fn home` and `src/lib.rs#fn home[2]`

### Requirement: Impl block labels

The label of an impl block SHALL be `impl` and the implemented type, such as `impl Engine`. For a trait impl, it SHALL
be `impl`, the trait, `for`, and the type, such as `impl Display for Engine`. Generic parameters after `impl`, a `where`
clause, `unsafe`, and the `!` of a negative impl SHALL NOT be part of the label.

#### Scenario: Trait impl

- **WHEN** `src/engine.rs` has `impl<T: Clone> From<T> for Engine where T: Send`
- **THEN** its node path is `src/engine.rs#impl From<T> for Engine`

#### Scenario: Inherent impl and trait impl of one type

- **WHEN** `src/engine.rs` has `impl Engine` and `impl Default for Engine`, each with a function `fn new`
- **THEN** the functions have the node paths `src/engine.rs#impl Engine/fn new` and
  `src/engine.rs#impl Default for Engine/fn new`

### Requirement: Extern block labels

The label of an extern block SHALL be the keyword `extern` and its ABI string when it has one, such as `extern "C"`.

#### Scenario: Function in an extern block

- **WHEN** `src/ffi.rs` has `extern "C" { fn abs(x: i32) -> i32; }`
- **THEN** the function has the node path `src/ffi.rs#extern "C"/fn abs`

### Requirement: Nesting of items

An item SHALL be a child of the inline module, impl block, trait, extern block, or function whose range contains it, by
the rule "Parent by containment" of the spec `outline`. An item at the top of the file SHALL be a child of the root
node.

#### Scenario: Function in an inline module

- **WHEN** `src/lib.rs` has `mod tests { fn helper() {} }`
- **THEN** the function has the node path `src/lib.rs#mod tests/fn helper`

#### Scenario: Items of a trait

- **WHEN** `src/scan.rs` has `trait Scan { type Output; const LIMIT: usize; fn scan(&self) -> Self::Output; }`
- **THEN** the outline has the nodes `src/scan.rs#trait Scan/type Output`, `src/scan.rs#trait Scan/const LIMIT`, and
  `src/scan.rs#trait Scan/fn scan`

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
  `src/entry.rs#enum Entry/Link/field`, and `src/entry.rs#enum Entry/Link/field[2]`

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
- **THEN** the range of the node `src/id.rs#struct Id/field` starts at `#[serde(skip)]`

### Requirement: Rust code that is not a node

`use` declarations, `extern crate` items, macro invocations, and the statements and expressions in a function body SHALL
NOT be nodes. Their text SHALL stay in the range of the node that contains them.

#### Scenario: Imports and statements

- **WHEN** `src/main.rs` has `use std::fs;` and `fn main() { let text = fs::read_to_string("a").unwrap(); }`
- **THEN** the outline has two nodes, `src/main.rs` and `src/main.rs#fn main`

### Requirement: Documented parser limit

The Rust plugin documentation SHALL identify the Rust 2024 `safe` item syntax in `unsafe extern` blocks that
tree-sitter-rust 0.24.2 rejects. It SHALL explain how this affects sync.

#### Scenario: Valid declaration rejected by the grammar

- **WHEN** a user looks up the diagnostic for `unsafe extern "C" { pub safe fn f(); }`
- **THEN** the documentation identifies the grammar limitation and explains that sync keeps the last good structure and
  exits with status 1
