---
id: TASK-18
title: Let readers customize source and outline colors
status: Done
assignee:
  - '@codex'
created_date: '2026-11-26 15:35'
updated_date: '2026-11-28 10:17'
labels: []
dependencies:
  - TASK-16
references:
  - >-
    backlog/tasks/task-14 -
    Support-XDG-plugins-and-settings-with-user-or-project-compilation-caches.md
  - openspec/changes/archive/2026-11-28-task-18-custom-colors
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
- [x] #1 Readers can change each source syntax color, including comments and headings, without rebuilding Ardent.
- [x] #2 Readers can color outline node kinds, including functions and headings, independently of source colors.
- [x] #3 Both selectors use the user theme directory. Omitted colors use the built-in palette.
- [x] #4 The selected outline row remains identifiable when its color matches another row.
- [x] #5 The guide includes a working theme example and explains how to apply it and restore the defaults.
- [x] #6 Switching named themes in config.yaml or editing the active palette preserves the reading position.
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
1. Install named palettes under the user configuration directory. The project theme selector overrides the user
   selector.
2. Read the selected global palette and use built-in colors for omitted entries.
3. Apply selector and active palette changes while browsing, preserving navigation and recovering from invalid saves.
4. Verify the guide example, live switching and defaults, run the repository gate and archive the change.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Review fixes: preserve the open document during selector errors and palette edits, accept Unix theme stems with colons,
and document YAML mapping order.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Readers can switch user-installed themes while browsing. Invalid selector edits keep the open document and show a
diagnostic until corrected. Palette edits repaint during a blocked project sync. Unix theme names accept colons, and the
schema key can appear anywhere in the theme mapping. All 239 tests passed across the workspace and the additional TUI
regression run. Rust formatting, Clippy, the build and all eight OpenSpec checks passed.
<!-- SECTION:FINAL_SUMMARY:END -->
