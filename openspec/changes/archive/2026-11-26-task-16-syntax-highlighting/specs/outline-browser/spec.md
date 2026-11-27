# Outline browser

## ADDED Requirements

### Requirement: Color outline labels by node kind

The browser SHALL use node kind to color outline labels and SHALL identify the selected row with a visible marker
independent of color.

#### Scenario: Move between node kinds

- **WHEN** a reader moves from a Markdown heading to its paragraph
- **THEN** the two labels have distinct colors and the selection marker moves to the paragraph

### Requirement: Keep source styles aligned with the selected passage

The browser SHALL style the current source bytes with the matching format plugin's syntax categories. Selecting a node,
wrapping a line, resizing the pane or paging SHALL preserve the text, existing display substitutions and the style of
each displayed grapheme. Uncovered source SHALL use the plain style.

#### Scenario: Page through a wrapped passage

- **WHEN** a reader selects a child after a multibyte character, narrows the pane so a colored token wraps, and pages
  through the passage
- **THEN** each visible part of the token retains its style, adjacent plain characters retain theirs, and the text
  occupies the same cells as in the plain preview

#### Scenario: Refresh the open file

- **WHEN** an edit moves a colored token inside the selected node and the browser reloads the source
- **THEN** the highlight follows the token to its new position
