# Tasks

Finish each item with passing `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.

## 1. Load files and their outlines

- [x] 1.1 Expose sync's tracked paths and file errors to the CLI browser. Test a project opened from a subdirectory with
      an ignored file, an unparseable file, and a healthy file. Check the existing sync summary and exit code.
- [x] 1.2 Load a structure and its source bytes for preview. Test multibyte text, an empty root, a missing file, an
      invalid range, and a retained structure after a same-length edit.

## 2. Navigate and render

- [x] 2.1 Add `ardent-tui` with terminal-independent outline navigation. Test expand, collapse, parent, child,
      visible-row and sibling movement, escaped delimiters, duplicate labels, equal ranges, and boundary moves.
- [x] 2.2 Wire `ardent browse` to the file list, outline, and scrollable preview. Document its keys and invocation in
      the README. Exercise file switching, a long passage, resize, safe display of control characters, quit, and
      terminal restoration using the CLI and a pseudo-terminal.

## 3. Refresh during a session

- [x] 3.1 Connect coalesced file events to sync and document live refresh and stale previews. Test editing the selected
      node, a failed scan and recovery, removing the open file, and ignoring the browser's own cache writes.
- [x] 3.2 Run the documented command on a project with Markdown and Rust. Verify its file list, navigation, byte-range
      previews, live refresh, and terminal exit. Format changed Markdown, review the diff, and run strict OpenSpec
      validation.

## Workflow follow-up

- Archive the change in the implementation PR, update the Backlog record and its reference, and validate the archived
  spec.
