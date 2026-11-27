# Markdown format

## ADDED Requirements

### Requirement: Markdown syntax categories

The packaged Markdown format plugin SHALL identify ATX and setext heading syntax as headings, and inline code spans,
fenced code blocks and indented code blocks as code.

#### Scenario: Prose beneath a heading

- **WHEN** a reader opens a document with a heading followed by a paragraph containing an inline code span
- **THEN** the heading has the heading style, the code span has the code style, and the surrounding paragraph has the
  plain style

#### Scenario: Read a code block

- **WHEN** a reader selects a fenced or indented code block
- **THEN** its source has the code style throughout the block
