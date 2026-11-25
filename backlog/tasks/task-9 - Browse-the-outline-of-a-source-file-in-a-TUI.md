---
id: TASK-9
title: Browse the outline of a source file in a TUI
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-25 04:04'
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
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added ardent browse. Select a tracked file, navigate its outline, and scroll the source text of the selected node. The
browser refreshes after edits and reports stale or unreadable files while healthy files stay available. The ardent-tui
crate owns navigation and rendering. Verified with 144 workspace tests, including state transitions and terminal
sessions on macOS with native notifications and polling. Build, format, Clippy, Markdown, and strict OpenSpec checks
passed.
<!-- SECTION:FINAL_SUMMARY:END -->
