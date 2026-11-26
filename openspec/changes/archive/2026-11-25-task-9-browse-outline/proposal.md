# Browse a source file's outline

## Why

Ardent already builds outlines for Markdown and Rust. TASK-9 lets a reader open a file, move through its outline, and
read the passage that each node covers.

## What Changes

- Add `ardent browse`. It runs sync and lists the project's tracked source files by path.
- Let the reader open a file, expand and collapse branches, move between nodes, and read the selected node's source
  text.
- Refresh the browser after external edits as specified in decision-2. If a scan fails, show the error and keep other
  files available. Use the source hash to detect stale ranges as described in decision-7.

Source editing, views, plugin dashboards, and external editor integration are outside TASK-9.

## Capabilities

### New Capabilities

- `outline-browser` covers file selection, keyboard navigation, source previews, and refresh during a terminal session.

### Modified Capabilities

None.

## Impact

The CLI starts the command and supplies its sync results. A new `ardent-tui` crate owns terminal rendering and
navigation state. The browser reads the existing structure format and uses the project's installed format plugins
through sync.
