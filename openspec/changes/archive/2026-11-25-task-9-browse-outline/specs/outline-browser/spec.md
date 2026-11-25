# Outline browser

## Purpose

The browser lets a reader choose a tracked source file, navigate its outline with the keyboard, and read the text that
belongs to a node.

## ADDED Requirements

### Requirement: Choose a tracked file

From any directory in a project, `ardent browse` SHALL run sync on entry and list the tracked source paths in path
order. Up and Down SHALL select a file, Enter SHALL open its outline, and Escape from an outline SHALL return to the
file list.

#### Scenario: Start in a subdirectory

- **WHEN** a reader runs `ardent browse` in `docs/` with two tracked files, `README.md` and `docs/guide.md`, and a
  git-ignored `draft.md`
- **THEN** the browser lists `README.md` and `docs/guide.md` in that order
- **AND** selecting `docs/guide.md` and pressing Enter opens its file node with its immediate children visible

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
- **AND** restoring read access to an unreadable tracked file lets the browser recover without another edit

#### Scenario: Change which files Git ignores

- **WHEN** an applicable Git ignore rule changes above the project root
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
