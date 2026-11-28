# Outline browser

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
- **AND** Shift+Tab switches focus without closing the file or losing the reader's place
- **AND** moving the explorer selection or collapsing a directory leaves the open document visible
- **AND** Escape closes the open document and focuses the explorer

#### Scenario: Use a narrow terminal

- **WHEN** the terminal has fewer than 60 columns
- **THEN** the focused pane uses the available width and Shift+Tab switches between the explorer and the open document
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

## ADDED Requirements

### Requirement: Toggle inline properties independently

With the outline focused, Tab SHALL toggle only the selected node’s inline properties. Each open node SHALL show one
indented `⚬ name: value` row per opted-in property directly below its label, before its children, even when its branch
is collapsed. Property rows SHALL be read-only and skipped by node navigation. The toggle SHALL preserve selection,
source preview scroll and child expansion.

#### Scenario: Compare two nodes

- **WHEN** the reader opens properties on one node, moves Down to the next visible node, and presses Tab again
- **THEN** both nodes show their own properties and the preview shows the second node
- **AND** pressing Tab again hides only the second node’s rows

#### Scenario: Inspect an expanded branch

- **WHEN** the reader pages through the selected branch’s source and presses Tab
- **THEN** its properties appear before its first child, the child stays visible and the preview stays at the same
  passage
- **AND** Down selects the first child

#### Scenario: A node has only a range

- **WHEN** the reader presses Tab on a node with a byte range and no opted-in properties
- **THEN** the outline gains no rows and its selection stays in place

#### Scenario: Return from the explorer

- **WHEN** the reader presses Shift+Tab twice with properties open, in either terminal layout
- **THEN** the reader returns to the same node, open rows, branch expansion and preview position

#### Scenario: Discover the bindings

- **WHEN** the reader opens the outline or consults the reader guide
- **THEN** the key hints and guide identify Tab for properties and Shift+Tab for switching panes
