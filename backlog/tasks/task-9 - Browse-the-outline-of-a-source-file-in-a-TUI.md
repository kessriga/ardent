---
id: TASK-9
title: Browse the outline of a source file in a TUI
status: To Do
assignee: []
created_date: '2026-11-19 16:29'
labels: []
milestone: m-1
dependencies:
  - TASK-7
priority: medium
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The outline is only useful if a person can move through it the way they move through Roam or Tana: expand and collapse nodes, move between siblings, and see the source text a node stands for. This is the first terminal UI of Ardent and sets the crate boundary between the document model and the interface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A command opens a TUI that lists the tracked source files of the project and opens the outline of the selected file
- [ ] #2 The user can expand and collapse nodes and move between parent, child, and sibling nodes with the keyboard
- [ ] #3 Selecting a node shows its source text, read from the source file through the node's source range
- [ ] #4 The TUI code lives outside ardent-core
- [ ] #5 Tests cover outline navigation as state transitions without a real terminal
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
