---
id: TASK-9
title: Browse the outline of a source file in a TUI
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-26 11:08'
labels: []
milestone: m-1
dependencies:
  - TASK-7
references:
  - openspec/changes/archive/2026-11-25-task-9-browse-outline
  - openspec/changes/archive/2026-11-26-task-9-sidebar-explorer
priority: medium
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The outline is only useful if a person can move through it the way they move through Roam or Tana: expand and collapse
nodes, move between siblings, and see the source text a node stands for. This is the first terminal UI of Ardent and
sets the crate boundary between the document model and the interface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A command opens a TUI that lists the project's tracked source files and opens the selected file's outline
- [x] #2 The user can expand and collapse nodes and move between parent, child, and sibling nodes with the keyboard
- [x] #3 Selecting a node shows its source text, read from the source file through the node's source range
- [x] #4 The TUI code lives outside ardent-core
- [x] #5 Tests cover outline navigation as state transitions without a real terminal
- [x] #6 The Ratatui browser groups tracked files by directory in a keyboard-accessible sidebar alongside the outline
      and source, and supports narrow terminals.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [x] #2 Changed Markdown is formatted with rumdl
- [x] #3 README and specs match the new behavior
- [x] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Use sync results to list tracked files and load a structure with matching source bytes.
2. Add terminal-independent navigation in ardent-tui and test parent, child, sibling, and expansion transitions.
3. Expose ardent browse, render the source preview, and refresh after external edits under decision-2.
4. Exercise the terminal command, update the README, run the project gate, and archive the OpenSpec change in the
   implementation PR.

5. Fix the four PR review findings. Verify quit under a held sync lock, continued browsing after a watcher path error,
   node selection after repair, and scroll position after an unrelated edit.

6. Pin browser sync to its project, recover from stale stamps and permission changes, watch ancestor ignore rules, and
   suppress refreshes from ignored build output. Verify all four watch cases and project isolation in temporary
   projects. Make diagnostics readable at any viewport size, handle CRLF, and keep quit visible in narrow terminals.

7. Track the displayed file through errors and closure, page file diagnostics, refresh nested Git excludes and resolved
   conflicts, validate regular files when opening sources, and check the terminal before starting sync. Exercise each
   case and run the project gate.

8. Add the directory explorer and independent pane focus to the existing browser. Use Ratatui with Crossterm, keep sync
   and preview recovery, exercise the real terminal, and update the specs and usage guide.

9. Match source and diagnostic wrapping to Ratatui grapheme widths, keep the file error marker visible in the sidebar,
   and verify both at the reported terminal sizes.

10. Add Just recipes for host checks and local installation. Verify the installed executable can initialize and sync a
    project with both bundled format plugins.

11. Honor Cargo’s install root, replace bundled format packages on reinstall, and protect Markdown snapshot inputs.
    Verify a configured install, an upgrade, and both directory and explicit formatting arguments.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added ardent browse for navigating tracked files, expanding outline nodes, and reading their source. Ratatui draws a
directory explorer beside the open outline and source. Tab changes focus. On narrow terminals, the focused pane fills
the screen. Sync preserves the explorer cursor, collapsed directories, and the open node. Scans, source updates, file
errors, and deletion stay connected to the open path while the reader navigates the explorer. The browser pages through
diagnostics, rejects redirected input before taking the sync lock, and restores the terminal on exit. Terminal tests
exercised native notifications and polling, nested directories, resize, refresh, and terminal restoration. Linux and
Windows passed compile checks. Source and diagnostics wrap at grapheme boundaries. The explorer places the scan-error
marker before each affected filename. The Justfile checks the workspace, honors Cargo’s configured install root, and
replaces bundled packages on reinstall. Markdown formatting skips snapshot inputs. All 173 Rust tests and both recipe
tests passed. A reinstalled release initialized and synced Markdown and Rust files.
<!-- SECTION:FINAL_SUMMARY:END -->
