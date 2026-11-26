---
id: TASK-18
title: Let readers customize source and outline colors
status: To Do
assignee: []
created_date: '2026-11-26 15:35'
updated_date: '2026-11-26 15:41'
labels: []
dependencies:
  - TASK-16
references:
  - >-
    backlog/tasks/task-14 -
    Support-XDG-plugins-and-settings-with-user-or-project-compilation-caches.md
type: enhancement
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Readers need colors that work with their terminal background. For example, a Markdown heading can appear in both the
source preview and the outline, and readers may want different colors in each.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Readers can change each source syntax color, including comments and headings, without rebuilding Ardent.
- [ ] #2 Readers can color outline node kinds, including functions and headings, independently of source colors.
- [ ] #3 For each color, check the project theme first, then the user theme, then the built-in palette.
- [ ] #4 The selected outline row remains identifiable when its color matches another row.
- [ ] #5 The guide includes a working theme example and explains how to apply it and restore the defaults.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
