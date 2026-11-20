# Proposal

## Why

`ardent sync` writes structure files only for Markdown, and it warns that the `rust` format of the initial configuration
has no plugin. Rust is the second built-in format of M1. It tests whether the document model fits source code as well as
prose: items nest inside modules, impl blocks, and traits; names repeat across a file; and doc comments and attributes
belong to the item below them. This change implements Backlog TASK-8.

## What Changes

- Add the built-in `rust` format plugin, version `0.1.0`, for `*.rs` files. Its nodes are the items of a file and their
  members:
  - items: modules, impl blocks, traits, functions (also function signatures), structs, enums, unions, constants,
    statics, type aliases, `macro_rules!` macros, and extern blocks;
  - members: struct, union, and variant fields, enum variants, and the associated functions, constants, and types of
    traits and impl blocks.
- Add the query capture `@annotation` (new decision-6). The range of a node covers the `@annotation` captures of its
  match. The Rust plugin captures the outer doc comments and attributes before an item, field, or variant this way, so
  they are part of its node.
- Report a syntax error (new decision-7). When the parse tree of a file contains an error, the outline engine returns no
  nodes and gives the line and column of the first error. The sync reports the file, keeps its last structure file, goes
  on with the other files, and exits with status 1.
- `ardent sync` tracks `*.rs` files with the initial configuration, and no longer warns that `rust` has no plugin.

Out of scope:

- Statements and expressions inside function bodies. A function body stays part of its function node.
- `use` declarations, `extern crate` items, and item-level macro invocations, such as `thread_local! { ... }`. Their
  text stays in the range of the parent node.
- Items that a macro generates, and items inside macro bodies. tree-sitter does not parse macro input.
- Modules in other files. A declaration `mod engine;` is a node, but Ardent does not follow it to `engine.rs`; that file
  has its own outline.
- An ordinary `//` comment between a doc comment and its item. It ends the run of annotations (decision-6).
- Search prefixes, project plugins, and the TUI.

## Capabilities

### New Capabilities

- `rust-format`: the built-in Rust format plugin: its name, version, default file patterns, node kinds, labels, and
  which doc comments and attributes belong to a node.

### Modified Capabilities

- `outline`: "Nodes from the outline query" gets the `@annotation` capture, which widens the range of a node. A new
  requirement states that a parse tree with a syntax error gives no outline.
- `sync`: "File errors" adds files with a syntax error, which keep their last structure file. The scenario "Format
  without a plugin" uses a format other than `rust`.

## Impact

- `ardent-format`: the outline engine reads `@annotation` captures and returns a syntax error. A new module `rust` holds
  the plugin and its `rust/outline.scm`.
- `ardent-cli`: the sync loads the Rust plugin next to the Markdown plugin.
- New dependency: `tree-sitter-rust`, pinned to `=0.24.2`. decision-3 names it as the grammar of the built-in Rust
  plugin, and its throwaway check used this version.
- `README.md`: the status line and the note about the missing Rust plugin change.
