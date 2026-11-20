# Spec Delta

## MODIFIED Requirements

### Requirement: Nodes from the outline query

Each syntax node that an `@node` capture of the outline query matches SHALL give one node of the outline. The byte range
of the capture SHALL be the range of the node, widened by the `@annotation` captures of the match (see "Annotation
captures"). A syntax node that more than one pattern matches SHALL give one node, from the first pattern that matches
it.

#### Scenario: Paragraph pattern

- **WHEN** a format plugin has the pattern `((paragraph) @node (#set! kind "paragraph"))` and a file has one paragraph
  from byte 0 to byte 49
- **THEN** the outline has one node with the kind `paragraph` and the range `0..49`

## ADDED Requirements

### Requirement: Annotation captures

An `@annotation` capture SHALL mark text that belongs to the node of its match, such as a doc comment. The range of the
node SHALL start at the smallest start and end at the largest end of the `@node` capture and the `@annotation` captures
of the match. The `@node` capture alone SHALL identify the node for the other rules of this spec.

#### Scenario: Doc comment before a function

- **WHEN** a format plugin has the pattern `((line_comment)* @annotation . (function_item) @node)`, and `src/lib.rs`
  holds the line `/// Runs.` and then the line `fn run() {}`
- **THEN** the node of the function has the range `0..21`

#### Scenario: Match without annotations

- **WHEN** the same pattern matches a function with no comment before it
- **THEN** the range of the node is the range of the function

### Requirement: Syntax errors

When the parse tree of a file contains a syntax error, Ardent SHALL build no outline for the file. It SHALL report the
line and the column of the first error in the file, both counted from 1, with the column counted in characters.

#### Scenario: Half-typed function

- **WHEN** `src/lib.rs` holds the line `fn run() {}` and then the line `fn helper(`
- **THEN** Ardent builds no outline for `src/lib.rs`, and reports a syntax error on line 2
