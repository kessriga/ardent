# Design

## Context

TASK-7 built the outline engine in `ardent-format` and the sync in `ardent-cli`. The engine is generic: all Markdown
knowledge is in `markdown/outline.scm`. The sync already keeps the old structure file when `FormatPlugin::scan` fails,
because it writes only after a scan succeeds. The initial `config.yaml` already enables `rust` with `*.rs`, and the sync
warns that `rust` has no plugin.

The accepted decisions fix the shape:

- decision-3: a format plugin is a grammar and an outline query. The built-in Rust grammar links natively through
  `tree-sitter-rust`.
- decision-6 (new): the `@annotation` capture widens the range of a node.
- decision-7 (new): a syntax error is a file error, and the sync keeps the last structure file.

A throwaway check in the session scratchpad ran `tree-sitter` 0.27.2 with `tree-sitter-rust` 0.24.2 on the queries
below. It confirmed the annotation runs, the labels, and the syntax error of a half-typed function.

## Goals / Non-Goals

**Goals:**

- All Rust knowledge stays in `rust/outline.scm`. The engine gets only the generic `@annotation` capture and the syntax
  error.
- The Rust example of the spec `ardent-schemas` keeps the paths, ranges, and fingerprints of its `impl` and `fn` nodes.

**Non-Goals:**

- A partial outline for a file with a syntax error (decision-7).
- A way to configure which items become nodes. A project plugin with its own query can do this later.

## Decisions

### Crates and modules

| Crate           | Module   | Change                                                                                  |
| --------------- | -------- | --------------------------------------------------------------------------------------- |
| `ardent-format` | `engine` | Reads `@annotation` captures. Returns `ScanError::Syntax` for a tree with an error.     |
| `ardent-format` | `rust`   | New. The built-in Rust plugin and its `rust/outline.scm`, embedded with `include_str!`. |
| `ardent-cli`    | `sync`   | Loads the Rust plugin next to the Markdown plugin.                                      |

`ardent-core` does not change. `ardent-format` gets `tree-sitter-rust = "=0.24.2"`. The exact pin keeps the grammar
version in the format hash equal to the linked grammar, as for `tree-sitter-md`.

The Markdown and the Rust plugin compute their format hash with the same rule (spec `ardent-schemas`, "Format hash").
The rule moves from `markdown.rs` into one function in `engine.rs`, next to `FormatPlugin`, and both plugins call it.
The test that checks the pin in `Cargo.toml` stays in each plugin module, because each pins its own crate.

### Annotation captures

`CompiledQuery` looks up the capture index of `annotation`, like `label`. For each match, the range of the candidate
runs from the smallest start to the largest end of the `@node` capture and the `@annotation` captures. The candidate
keeps the `@node` syntax node for the rule "first pattern wins" and for its depth. Nothing else in the engine changes:
nesting by containment and by level uses the widened range.

### Syntax errors

`FormatPlugin::scan` checks `has_error` on the root of the parse tree before it runs the query. When the tree has an
error, `scan` returns `ScanError::Syntax { line, column }` and no nodes. To find the first error, the engine starts at
the root and moves down to the first child that has an error, until it reaches an `ERROR` or a `MISSING` node. When no
child has an error, the node itself is the error. The walk is a loop, not a recursion, so a deep tree cannot overflow
the stack. The line is the tree-sitter row plus 1. The column is the number of characters from the start of the line to
the error, plus 1, so that it matches what an editor shows for a line with non-ASCII text.

The sync needs no new code: `sync_file` returns the error before it writes, so the structure file stays as it was, and
`run` reports `<path>: syntax error at line <l>, column <c>`.

Alternative: return the recovered nodes together with the error, and let the sync choose. No caller needs the nodes
(decision-7), so the engine does not build them. Rejected.

### Rust query

A pattern for an item has this shape:

```scheme
(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (function_item "fn" @label name: (_) @label) @node
  (#set! kind "fn"))
```

- The pattern names no parent node. In a pattern with a parent, a run of zero annotations followed by the anchor `.`
  matches only the first child of the parent, so items after the first one without annotations would not match.
- Labels capture the keyword and the name. A trait impl captures `impl`, the trait, `for`, and the type. The `!` of a
  negative impl is not captured, because the engine joins label captures with a space and would give `impl ! Send`.
- Fields of tuple structs and tuple variants are bare type nodes in `ordered_field_declaration_list`, so their pattern
  must name that parent. Two patterns cover them: first one with one or more annotations (attributes and the
  visibility), then one without. The engine keeps the first pattern that matches a syntax node.
- Function signatures (`function_signature_item`) in traits and extern blocks get the kind `fn`. Associated types
  (`associated_type`) in traits and type aliases (`type_item`) in impl blocks get the kind `type`. Constants in traits
  and impl blocks are `const_item`, like top-level constants.

### Built-in plugin metadata

The Rust plugin follows the Markdown plugin: name `rust`, version `0.1.0`, default pattern `*.rs`, and the format hash
of the query text, then `tree-sitter-rust`, then `0.24.2`.

### Tests

- `ardent-format` engine: a pattern with annotations widens the range; a match without annotations keeps the `@node`
  range; a tree with an error gives `ScanError::Syntax` with the right line and character column.
- `ardent-format` Rust plugin: identity, the pin, the spec example of `ardent-schemas` (its `impl` and `fn` nodes with
  their ranges and fingerprints, plus `struct Engine` and `files`), and one test for each scenario of the spec
  `rust-format`.
- `ardent-format` snapshots: `insta` snapshots for Rust files in `tests/rust/` with inline modules, impl blocks, trait
  impls, fields, variants, and doc comments, and a snapshot of the error message for a file with a syntax error. The
  files keep a name other than `main.rs`, so that Cargo does not build them as a test target.
- `ardent-cli`: integration tests for a sync of Rust files, a syntax error in one of two files, and a syntax error after
  a scan. The warning test enables `html` in place of `rust`.

## Risks / Trade-offs

- [A Markdown file that `tree-sitter-md` parses with an error now fails, where it gave an outline before] → The block
  grammar of `tree-sitter-md` accepts every input in the existing tests. The verification syncs the Markdown files of
  this repository. A failure shows up as a file error with a position, not as lost data.
- [A Rust project mid-edit makes `ardent sync` exit with status 1] → decision-7 accepts this. The error names the file
  and the position, and the structure file stays as it was.
- [An ordinary comment between a doc comment and its item ends the annotation run] → decision-6 accepts this. A later
  query version can include ordinary comments, and its format hash re-scans the files.
- [Tuple fields have positional paths such as `field[2]`, which shift when a person inserts a field before them] → The
  same weak spot as Markdown list items (decision-1). Re-scans keep stored paths current once the matcher exists.
- [Two items with the same label under one parent, such as two `cfg` variants of `fn home`, get `[2]`] → The spec
  `outline` defines this. The fingerprint tells them apart.

## Migration Plan

None. Projects from `ardent init` already enable `rust`, so their next sync tracks `*.rs` files and writes their
structure files.
