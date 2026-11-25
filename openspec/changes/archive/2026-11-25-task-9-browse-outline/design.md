# Design

## Context

The sync in `ardent-cli` already discovers the project root and sorts tracked files. Its report exposes only their
count. The structure in `ardent-core` holds a flat outline in document order, with a node path and byte range for each
node. Equal ranges are valid for a parent and child. Decision-1 defines escaped node paths. Decisions 2, 4, and 7 govern
refresh, cache access, and scan failures.

## Goals and boundaries

Keep project discovery, sync, and file reads in `ardent-cli`. Put terminal input, rendering, and navigation state in
`ardent-tui`, which depends on `ardent-core`. The CLI depends on both. The terminal crate can accept an outline and
source bytes from the CLI without loading a format plugin.

## Decisions

- **File list.** Return the sorted paths from the existing sync walk to the browser. Preserve the summary and exit
  behavior of `ardent sync`. Listing the cache can include structures kept after a failed scan and miss tracked files
  that have never parsed. Report file errors beside the affected path and keep healthy files available.
- **Tree.** Decode the hierarchy from node paths once per open file. Treat an escaped delimiter as label text and retain
  positional suffixes for duplicate siblings. Use parent and sibling indices for movement and keep expansion and
  selection in memory. Recovering parents from ranges would give the wrong result for nodes with identical ranges.
- **Controls.** Use the arrow keys for visible rows and parent or child navigation, brackets for siblings, and Page Up
  and Page Down to scroll the preview. Keep the selected row visible when a list exceeds the screen height. Route keys
  to state transitions that tests can call without a terminal.
- **Preview.** On open and after refresh, read the selected file once and compare its content hash with the structure
  stamp. Take checked byte slices from those same bytes as selection changes. Show a diagnostic for an unreadable file,
  a stale stamp, or an invalid range. Render arbitrary bytes as display text only after slicing. Escape terminal
  controls in all project-provided text.
- **Refresh.** Have `ardent-cli` watch the project and coalesce bursts of source changes before reusing sync. The
  existing incremental scan skips unchanged files. Filter generated cache events so writes by sync cannot trigger
  another refresh. Release the sync lock before waiting for input. Restore selection by source path and node path after
  a refresh.
- **Terminal.** Use a terminal event and drawing backend in `ardent-tui` that handles keys and resize events on the
  supported platforms. The existing init picker reads one short list and has a Unix-specific input loop. Leave that
  command independent. Scope terminal setup and cleanup to the browser session, including partial setup failures.

## Risks and checks

- A rename can produce separate remove and create events. Reconcile against the next sync's tracked list and test
  removal while the file is open.
- A syntax error can leave cached ranges that still fit inside the edited file. Test a same-length edit to prove the
  preview checks content identity.
- Duplicate labels, slashes in labels, and directories in source paths can confuse navigation. Exercise these cases
  together with equal parent and child ranges in state-transition tests.
