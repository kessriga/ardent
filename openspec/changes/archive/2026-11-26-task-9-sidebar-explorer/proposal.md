# Proposal

## Why

Readers need to browse directories while keeping their place in an open outline. Decision-9 describes a built-in file
explorer that groups tracked files by directory.

## What Changes

Keep a directory explorer beside the document at normal terminal widths. Let the reader move focus, open a different
file, and navigate the outline and its source with the keyboard. On narrow terminals, show the focused pane at full
width.

## Capabilities

### Modified Capabilities

- `outline-browser` gains a persistent explorer and keyboard focus for its panes.

## Impact

The change affects `ardent-tui`, the browser's terminal tests, and the usage guide. Ratatui draws the interface through
its Crossterm backend.
