# outline-browser Specification

## Purpose

The browser lets a reader choose a tracked source file, navigate its outline with the keyboard, and read the text that
belongs to a node.

## Requirements

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

### Requirement: Navigate the outline

Up and Down SHALL select the previous and next visible node in document order. Right SHALL expand a closed branch or
select the first child of an open branch. Left SHALL collapse an open branch or select the parent. The browser SHALL
also provide keys for the previous and next sibling, skipping descendants. A move without a destination SHALL keep the
selection.

#### Scenario: Expand a branch and select its child

- **WHEN** the reader presses Right twice on a collapsed heading with two children
- **THEN** the first press reveals its children and the second selects its first child
- **AND** Left from that leaf selects the heading, where another Left hides its children

#### Scenario: Skip an expanded sibling's children

- **WHEN** the reader uses next sibling on an expanded heading followed by another heading with the same parent
- **THEN** the second heading becomes selected
- **AND** previous sibling returns to the first heading

### Requirement: Read the selected source range

The browser SHALL display the selected node's half-open byte range from source bytes matching the structure's content
hash. The reader SHALL be able to scroll a passage that exceeds the preview area. The browser SHALL render control
characters in source text and labels safely.

#### Scenario: Select one of two passages

- **WHEN** a reader selects a paragraph after a multibyte character in a file containing two paragraphs
- **THEN** the preview shows the selected paragraph from its first byte through the byte before its end, excluding the
  adjacent paragraph

#### Scenario: A source file no longer matches its structure

- **WHEN** a file has a syntax error and sync retained its last good structure
- **THEN** the browser identifies the file and reports the error, withholds its stale preview, and lets the reader open
  another tracked file
- **AND** the reader can page through the full diagnostic on a narrow terminal

### Requirement: Refresh after source changes

While the browser is open, it SHALL sync after external changes to tracked files and update the file list, outline, and
preview to reflect the result. A removed selection SHALL fall back to its file node, or to the file list if the file
disappeared.

#### Scenario: Edit a selected paragraph

- **WHEN** a reader changes a selected paragraph in an editor and the node path survives the sync
- **THEN** the browser keeps the paragraph selected and displays its new source text

#### Scenario: Restore a file or its read permission

- **WHEN** a reader replaces an open source file with different content of the same size and modification time
- **THEN** the browser checks its content and displays the updated outline and preview
- **AND** the browser detects a repair that preserves the stamp while the file’s scan error is on screen
- **AND** restoring read access to an unreadable tracked file lets the browser recover without another edit

#### Scenario: Change which files Git ignores

- **WHEN** an applicable Git ignore rule changes above the project root or in a nested Git worktree’s external exclude
  file
- **THEN** the browser updates the file list
- **AND** subsequent writes inside an ignored build directory do not trigger a sync

#### Scenario: Lose the project directory

- **WHEN** the open project's `.ardent/` directory disappears
- **THEN** the browser reports the error and leaves any enclosing project's cache untouched

#### Scenario: Remove an open file

- **WHEN** a reader removes the open source file and the browser receives the change
- **THEN** the browser returns to the file list and the removed path is absent

### Requirement: Leave the terminal usable

The browser SHALL show its navigation and exit keys. Pressing q SHALL exit from either screen. The command SHALL restore
the terminal on quit, interruption, or a reported terminal error.

#### Scenario: Quit from the outline

- **WHEN** the reader opens an outline and presses q
- **THEN** the command exits and the shell accepts and echoes input normally

### Requirement: Open the browser by default

Running `ardent` without arguments SHALL open the same terminal browser as `ardent browse`. Running `ardent --help`
SHALL print the command list and exit successfully.

#### Scenario: Open an initialized project

- **WHEN** a reader runs `ardent` in an initialized project in an interactive terminal
- **THEN** the browser opens the explorer

#### Scenario: Discover commands

- **WHEN** a reader runs `ardent --help` outside a project with redirected output
- **THEN** the output lists the init, sync and browse commands

### Requirement: Distinguish outline and source panes

The browser SHALL draw an unlabeled horizontal divider across the document pane between the outline and source preview
when there is room for both panes and the divider. The divider SHALL leave the explorer and source paging controls
usable.

#### Scenario: Read an outline

- **WHEN** a reader opens a document beside the explorer
- **THEN** the reader sees the source below a horizontal divider that spans only the document pane
- **AND** the divider remains visible when the reader switches to the document in a narrow terminal with room for the
  divider

### Requirement: Display sibling positions

The browser SHALL display the one-based dotted position for every sibling in a repeated label group across all formats.
A singleton SHALL display its original label.

#### Scenario: Read a nested list

- **WHEN** a reader expands a two-item list whose first item contains a one-item list
- **THEN** the outer list shows `item.1` and `item.2`, and the inner list shows `item`
- **AND** selecting each row shows the source for its node path

#### Scenario: Read repeated names

- **WHEN** two sibling headings are named Install and another is named Usage
- **THEN** the outline displays `Install.1`, `Install.2`, and `Usage`
