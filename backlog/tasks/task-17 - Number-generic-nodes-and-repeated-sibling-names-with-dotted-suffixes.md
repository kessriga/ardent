---
id: TASK-17
title: Number generic nodes and repeated sibling names with dotted suffixes
status: Done
assignee:
  - '@codex'
created_date: '2026-11-26 13:26'
updated_date: '2026-11-26 13:33'
labels: []
dependencies: []
references:
  - openspec/changes/archive/2026-11-26-task-17-dotted-node-paths
type: enhancement
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use one node-path rule across format plugins. Readers see numbers when sibling labels repeat and short labels when they
are unique.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Generic labels always store .1, .2, and so on within each parent.
- [x] #2 Unique names stay bare. Repeated names receive .1, .2, and so on in document order.
- [x] #3 The browser hides a singleton index and shows every index in a repeated group.
- [x] #4 Literal dotted names remain distinct from generated indices.
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
1. Define generic labels, sibling numbering and literal-dot escaping in the specs. 2. Build paths and display labels
   together in core and use them in the browser. 3. Verify both plugins, cached structures and terminal navigation.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Core builds dotted node paths and display labels for every plugin. Sync rebuilt the four cached example structures once.
Verified repeated headings, generic singletons, literal dots, nested lists and Rust fields. The full just check gate and
strict spec validation passed.
<!-- SECTION:FINAL_SUMMARY:END -->
