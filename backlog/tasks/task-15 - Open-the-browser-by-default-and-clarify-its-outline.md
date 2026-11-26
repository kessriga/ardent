---
id: TASK-15
title: Open the browser by default and clarify its outline
status: Done
assignee:
  - '@codex'
created_date: '2026-11-26 12:34'
updated_date: '2026-11-26 12:38'
labels: []
dependencies: []
references:
  - openspec/changes/archive/2026-11-26-task-15-outline-browser
type: enhancement
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make the terminal browser the default command, separate the outline from the source, and display consistent numbers for
items in a list.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Running ardent opens the browser and --help lists commands.
- [x] #2 A horizontal divider separates the outline and source while browsing a file.
- [x] #3 Each item in a multi-item list displays a one-based index. A lone item displays as item.
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
1. Route the default CLI invocation to the browser and verify help.
2. Draw the source divider and number repeated list items in the outline display.
3. Test terminal behavior and nested lists, update the guide and specs, and run the repository gate.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Bare ardent opens the browser. Readers can see the boundary above the source and the position of each item in a
multi-item list. Verified the default command in the examples project, the divider at two terminal widths, and source
selection in a nested list. All 12 browser tests and the full just check gate passed.
<!-- SECTION:FINAL_SUMMARY:END -->
