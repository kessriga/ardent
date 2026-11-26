# Design

## Context

The CLI supplies the paths and diagnostics from each sync. Decision-9 makes Ardent responsible for layout, focus, and
navigation in the file explorer.

## Decisions

- Derive directory rows from the tracked paths. This keeps the explorer consistent with sync and Git ignore rules.
- Track the explorer selection separately from the open document. Moving through directories preserves the selected node
  and source position until the reader opens another file or closes the document.
- Use Ratatui for pane layout and drawing. Keep Crossterm for keyboard input and terminal mode restoration.
- At widths below 60 columns, give the focused pane the full width. Tab lets readers reach both panes even when only one
  fits.

## Risks

A refresh can remove either the open file or a highlighted directory. Restore each selection by path and fall back to a
surviving explorer entry when necessary. Test the visible terminal after incremental redraws and resizing.
