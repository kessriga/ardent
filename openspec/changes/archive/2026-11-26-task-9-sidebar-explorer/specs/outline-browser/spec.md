# Spec Delta

## MODIFIED Requirements

### Requirement: Choose a tracked file

From any directory in a project, `ardent browse` SHALL sync on entry and show an explorer of tracked files grouped by
directory. Directories SHALL start expanded and precede files among siblings, with each group sorted by name. Up and
Down SHALL move among visible entries. Enter SHALL toggle a directory or open a file at its file node. Right SHALL
expand a directory or select its first child. Left SHALL collapse an expanded directory or select the parent.

#### Scenario: Start in a subdirectory

- **WHEN** a reader runs `ardent browse` in `docs/` with tracked files `README.md` and `docs/guide.md` and a git-ignored
  `draft.md`
- **THEN** the explorer lists `docs`, its child `guide.md`, and `README.md`
- **AND** opening `guide.md` selects its file node and shows its immediate children and source

#### Scenario: Explore while reading

- **WHEN** a reader opens a file in a terminal at least 60 columns wide
- **THEN** the explorer stays on the left and the document pane displays the outline and the selected node's source
- **AND** Tab switches focus without closing the file or losing the reader's place
- **AND** moving the explorer selection or collapsing a directory leaves the open document visible
- **AND** Escape closes the open document and focuses the explorer

#### Scenario: Use a narrow terminal

- **WHEN** the terminal has fewer than 60 columns
- **THEN** the focused pane uses the available width and Tab switches between the explorer and the open document
- **AND** widening the terminal restores both panes with their selections intact

#### Scenario: Refresh independent selections

- **WHEN** sync updates the explorer while a different file is open
- **THEN** surviving explorer selections and collapsed directories keep their state, and the open document refreshes by
  its own path
- **AND** removing the open file closes the document and focuses the explorer

#### Scenario: Redirected input or output

- **WHEN** standard input or output is redirected
- **THEN** the command reports that it requires an interactive terminal before starting sync

#### Scenario: Read project diagnostics

- **WHEN** the file list contains diagnostics that exceed the terminal width or height
- **THEN** the browser wraps their text and the reader can page through every message

#### Scenario: Empty project

- **WHEN** sync finds no tracked source files
- **THEN** the browser reports that the file list is empty and accepts the quit key
