---
id: TASK-9
title: Browse the outline of a source file in a TUI
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-26 09:36'
labels: []
milestone: m-1
dependencies:
  - TASK-7
references:
  - openspec/changes/archive/2026-11-25-task-9-browse-outline
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
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added ardent browse for navigating tracked files, expanding outline nodes, and reading their source. The browser keeps
its project root, restores the selected node and reading position, and stays responsive while sync waits for the lock.
Project and file diagnostics wrap and page. Polling detects repairs to an open file even when its timestamp and size
stay the same, and stops checking the file when it closes. The picker refreshes when nested Git worktree exclusions
change or a format conflict disappears. Sync and preview reject named pipes without waiting. Redirected browse exits
before sync touches the cache. All 166 workspace tests passed on macOS. Terminal checks covered native notifications and
polling. The CLI passed compile checks for Linux and Windows. Build, formatting, Clippy, Rust API documentation, and
strict OpenSpec validation passed.
<!-- SECTION:FINAL_SUMMARY:END -->
