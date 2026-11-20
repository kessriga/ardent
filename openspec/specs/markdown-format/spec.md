# markdown-format Specification

## Purpose

Defines the built-in Markdown format plugin: its name, version, and default file patterns, which Markdown blocks become
nodes of the outline, their kinds and labels, and how headings nest.

## Requirements

### Requirement: Plugin identity

The built-in Markdown format plugin SHALL have the name `markdown` and the version `0.1.0`. Its default file patterns
SHALL be `*.md` and `*.markdown`.

#### Scenario: Stamp of a Markdown file

- **WHEN** a sync scans `README.md` with the built-in Markdown plugin
- **THEN** the stamp of its structure file has `format: markdown` and `format_version: 0.1.0`

### Requirement: Headings

Each ATX heading and each setext heading SHALL be a node of the kind `heading`. Its label SHALL be the text of the
heading, without the `#` markers or the underline. An ATX heading SHALL have the level of its number of `#` markers. A
setext heading SHALL have level 1 when it is underlined with `=` and level 2 when it is underlined with `-`.

#### Scenario: Structure of the spec example

- **WHEN** a sync scans the Markdown example of the spec `ardent-schemas`, under "Structure nodes"
- **THEN** the paths, kinds, ranges, and fingerprints of its nodes equal those of the example

#### Scenario: Setext heading

- **WHEN** `guide.md` has a setext heading `Setup` underlined with `---`, followed by a paragraph and then by the ATX
  heading `## Usage`
- **THEN** the paragraph has the node path `guide.md#Setup/paragraph`
- **AND** the ATX heading has the node path `guide.md#Usage`

#### Scenario: Heading without text

- **WHEN** `notes.md` has a line `##` with no text after the marker
- **THEN** the line gives a node with the label `heading`

### Requirement: Nesting of headings

A heading SHALL contain the blocks after it up to the next heading with the same or a smaller level, by the rule
"Nesting by level" of the spec `outline`. A block before the first heading SHALL be a child of the root node.

#### Scenario: Skipped heading level

- **WHEN** `guide.md` has `# Guide`, then `### Details`, then `## Install`
- **THEN** `### Details` and `## Install` are both children of `# Guide`, with the node paths `guide.md#Guide/Details`
  and `guide.md#Guide/Install`

#### Scenario: Setext heading of level 1 inside an ATX section

- **WHEN** `guide.md` has `## Old`, then a setext heading `New` underlined with `===`, then a paragraph
- **THEN** `New` is a sibling of `Old`, and the paragraph has the node path `guide.md#New/paragraph`

### Requirement: Paragraphs

A paragraph SHALL be a node of the kind `paragraph` when it stands before the first heading of the file or directly in
the content of a heading, also of a heading inside a list item or a block quote. Every other paragraph, such as the text
of a list item, a block quote, or a setext heading, SHALL be part of the node that contains it and SHALL NOT be a node.

#### Scenario: Paragraph under a heading

- **WHEN** the heading `## Install` in `README.md` is followed by one paragraph
- **THEN** the paragraph has the node path `README.md#Install/paragraph`

#### Scenario: Text of a list item

- **WHEN** a list item in `README.md` is `- Download the binary.`
- **THEN** the outline has no `paragraph` node inside the item

#### Scenario: Heading inside a list item

- **WHEN** a list item in `README.md` holds the heading `## Note` and a paragraph below it
- **THEN** the paragraph is a node, with the node path `README.md#list/item/Note/paragraph`

### Requirement: Lists and items

Each list SHALL be a node of the kind `list`, and each list item SHALL be a node of the kind `item`, as a child of its
list. A list, code block, block quote, or table inside a list item SHALL be a child of that item.

#### Scenario: Nested list

- **WHEN** the second item of the first list under `## Install` in `README.md` holds a nested list with two items
- **THEN** the nested items have the node paths `README.md#Install/list/item[2]/list/item` and
  `README.md#Install/list/item[2]/list/item[2]`

#### Scenario: Ordered list

- **WHEN** a list under `## Install` in `README.md` uses the markers `1.` and `2.`
- **THEN** it is a node of the kind `list`, and its items are nodes of the kind `item`

### Requirement: Code blocks

Each fenced code block and each indented code block SHALL be a node of the kind `code`, with the label `code`.

#### Scenario: Fenced code block with a language

- **WHEN** the heading `## Install` in `README.md` holds a code block that starts with the fence ```` ```sh ````
- **THEN** the code block has the node path `README.md#Install/code`

### Requirement: Block quotes and tables

Each block quote SHALL be a node of the kind `quote`, and each pipe table SHALL be a node of the kind `table`.

#### Scenario: Quote and table under a heading

- **WHEN** the heading `## Notes` in `README.md` holds a block quote and then a table
- **THEN** they have the node paths `README.md#Notes/quote` and `README.md#Notes/table`

### Requirement: Front matter

A YAML front matter block between `---` lines, or a TOML front matter block between `+++` lines, at the start of the
file SHALL be a node of the kind `front_matter`, as a child of the root node.

#### Scenario: YAML front matter

- **WHEN** `guide.md` starts with `---`, `title: Guide`, and `---`
- **THEN** the outline has the node `guide.md#front_matter` with the range of those three lines

### Requirement: Blocks that are not nodes

HTML blocks, thematic breaks, and link reference definitions SHALL NOT be nodes. Their text SHALL stay in the range of
the node that contains them.

#### Scenario: Thematic break under a heading

- **WHEN** the heading `## Notes` in `README.md` holds a paragraph, a line `---` that is a thematic break, and a second
  paragraph
- **THEN** `README.md#Notes` has two children, `paragraph` and `paragraph[2]`, and the range of the heading includes the
  thematic break
