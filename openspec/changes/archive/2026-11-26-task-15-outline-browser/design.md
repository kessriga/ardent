# Design

## Command dispatch

Make the subcommand optional in the CLI parser. When omitted, dispatch to the existing browse command so project
discovery, terminal checks and cleanup follow the same path. Let the parser handle help before dispatch.

## Terminal layout and labels

Draw the divider on a single row across the document pane. Short terminals need the remaining rows for the selected node
and its source. Keep keyboard focus shared between the outline and source so Page Down still scrolls the preview.

Decision 1 defines node paths and their positional suffixes. The TUI can recognize a multi-item list from its second
item and display a suffix on the first item's row. Navigation and refresh continue to use each node's stored path.
