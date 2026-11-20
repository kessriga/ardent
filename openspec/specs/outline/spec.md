# outline Specification

## Purpose

Defines how Ardent builds the outline of a source file from the outline query of a format plugin: which nodes the query
finds, their kinds and labels, how they nest, and how Ardent turns their labels into node paths. The rules follow
decision-1, decision-3, and decision-5.

## Requirements

### Requirement: Nodes from the outline query

Each syntax node that an `@node` capture of the outline query matches SHALL give one node of the outline. The byte range
of the capture SHALL be the range of the node. A syntax node that more than one pattern matches SHALL give one node,
from the first pattern that matches it.

#### Scenario: Paragraph pattern

- **WHEN** a format plugin has the pattern `((paragraph) @node (#set! kind "paragraph"))` and a file has one paragraph
  from byte 0 to byte 49
- **THEN** the outline has one node with the kind `paragraph` and the range `0..49`

### Requirement: Node kind

The property `kind` of a pattern SHALL give the kind of its nodes. Without the property, the kind SHALL be the
tree-sitter type of the captured syntax node.

#### Scenario: Pattern without a kind

- **WHEN** a pattern is `(block_quote) @node` and it has no `kind` property
- **THEN** its nodes have the kind `block_quote`

### Requirement: Node label

Ardent SHALL join the source text of all `@label` captures of one match, in document order, with one space. It SHALL
replace each run of ASCII whitespace with one space and remove spaces at the start and end. When a match has no `@label`
capture, or the result is empty, the label SHALL be the kind.

#### Scenario: Label with inline markup

- **WHEN** the label capture of a heading covers the text `Sync **rules**`
- **THEN** the label of the node is `Sync **rules**`

#### Scenario: Label over two lines

- **WHEN** the label capture of a heading covers `Setext` and `title` on two lines
- **THEN** the label of the node is `Setext title`

#### Scenario: Node without a label

- **WHEN** a match of the kind `paragraph` has no `@label` capture
- **THEN** the label of the node is `paragraph`

### Requirement: Label ranges

The byte ranges of the `@label` captures of a match SHALL be the label ranges of its node. The fingerprint of the node
SHALL leave out these ranges, as the spec `ardent-schemas` defines under "Fingerprint".

#### Scenario: Renamed heading

- **WHEN** a person renames the heading `## Sync rules` to `## Sync order` and changes nothing else
- **THEN** the node of the heading keeps its fingerprint

### Requirement: Root node

Every outline SHALL start with a root node for the whole file. The root node SHALL have the kind `file`, the range of
the whole file, and the source path as its node path.

#### Scenario: Empty file

- **WHEN** Ardent builds the outline of an empty tracked file `notes.md`
- **THEN** the outline has one node, with the path `notes.md`, the kind `file`, and the range `0..0`

### Requirement: Parent by containment

The parent of a node SHALL be the nearest other node whose range contains the range of the node. A node that no other
node contains SHALL be a child of the root node.

#### Scenario: Item inside a list

- **WHEN** a list has the range `89..245` and one of its items has the range `112..223`
- **THEN** the item is a child of the list

### Requirement: Nesting by level

The property `level` of a pattern SHALL be a positive integer. A node with a level SHALL contain the siblings after it,
up to the next sibling with the same or a smaller level, and its range SHALL end where that sibling starts, or at the
end of its parent when no such sibling follows. Ardent SHALL reject an outline query with a `level` that is not a
positive integer.

#### Scenario: Deeper level

- **WHEN** a file has, in order, a node with level 2, a paragraph, a node with level 3, a paragraph, and a node with
  level 2
- **THEN** the first level-2 node contains the first paragraph and the level-3 node
- **AND** the level-3 node contains the second paragraph
- **AND** the range of the level-3 node ends where the second level-2 node starts

#### Scenario: Skipped level

- **WHEN** a file has, in order, a node with level 1, a node with level 3, and a node with level 2
- **THEN** the level-1 node contains both other nodes, and the level-3 and the level-2 node are siblings

#### Scenario: Invalid level

- **WHEN** an outline query has `(#set! level "two")`
- **THEN** Ardent reports the format plugin and the invalid value, and builds no outline with that query

### Requirement: Node path

The node path of a node SHALL be the source path, `#`, and the labels of the nodes from the root down to the node,
joined by `/`. A backslash SHALL escape each `/`, `#`, `[`, `]`, and `\` in a label, and each `#`, `[`, `]`, and `\` in
the source path.

#### Scenario: Nested heading

- **WHEN** the heading `Renames` is a child of the heading `Sync rules` in `docs/spec.md`
- **THEN** its node path is `docs/spec.md#Sync rules/Renames`

#### Scenario: Slash in a label

- **WHEN** `docs/spec.md` has the heading `## Read/write`
- **THEN** its node path is `docs/spec.md#Read\/write`

#### Scenario: Hash sign in a source path

- **WHEN** the tracked file `docs/C#.md` has the heading `## Intro`
- **THEN** its node path is `docs/C\#.md#Intro`

### Requirement: Positional suffixes

When two or more children of one parent have the same label, the first child in document order SHALL keep the label in
its node path, and the next ones SHALL get the suffixes `[2]`, `[3]`, and so on.

#### Scenario: Two paragraphs

- **WHEN** the heading `Sync rules` in `docs/spec.md` has two paragraphs as children
- **THEN** their node paths are `docs/spec.md#Sync rules/paragraph` and `docs/spec.md#Sync rules/paragraph[2]`

#### Scenario: Two headings with one name

- **WHEN** `README.md` has two headings `## Install` with the same parent
- **THEN** their node paths are `README.md#Install` and `README.md#Install[2]`
