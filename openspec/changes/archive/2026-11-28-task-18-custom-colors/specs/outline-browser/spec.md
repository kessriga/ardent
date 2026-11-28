# Browser colors

## MODIFIED Requirements

### Requirement: Color outline labels by node kind

The browser SHALL use the resolved outline color for each node kind and SHALL mark the selected row with a visible `>`
independent of color.

#### Scenario: Move between node kinds

- **WHEN** a reader using the built-in palette moves from a Markdown heading to its paragraph
- **THEN** the two labels have distinct colors and the selection marker moves to the paragraph

#### Scenario: Two kinds share a color

- **WHEN** the reader sets the same color for headings and paragraphs and moves between them
- **THEN** both labels keep that color and only the selected row has the selection marker

## ADDED Requirements

### Requirement: Apply source colors by syntax category

The browser SHALL use the resolved source color for each highlighted span and the plain style for uncovered text. It
SHALL apply the source and outline colors independently, using the active theme.

#### Scenario: Read a heading in both panes

- **WHEN** the reader selects a Markdown heading with different source and outline heading colors
- **THEN** the heading text in the source preview and its label in the outline each use their configured foreground

### Requirement: Change the theme while browsing

On the next refresh after a selector or active theme file changes, the browser SHALL apply the resolved palette to both
panes and preserve the open document, selected node and scroll position. If the theme cannot load, the browser SHALL
show the diagnostic and keep the last usable palette, using built-in colors if none has loaded yet.

#### Scenario: Switch between saved themes

- **WHEN** the reader changes `theme: catppuccin` to `theme: daylight` while reading a function
- **THEN** the visible source and outline use the daylight colors at the same reading position

#### Scenario: Edit the active theme

- **WHEN** the reader saves a new source heading color in the selected theme file
- **THEN** the visible heading changes on refresh and the outline keeps its configured color

#### Scenario: Recover from an incomplete save

- **WHEN** the selected palette becomes invalid and the reader then saves a valid palette at the same path
- **THEN** the browser keeps its previous colors while showing the error, then applies the corrected colors and clears
  the error
