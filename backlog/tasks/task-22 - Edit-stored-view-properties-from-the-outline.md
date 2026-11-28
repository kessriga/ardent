---
id: TASK-22
title: Edit stored view properties from the outline
status: To Do
assignee: []
created_date: '2026-11-28 12:11'
labels: []
dependencies:
  - TASK-21
references:
  - openspec/specs/ardent-schemas/spec.md
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Readers should be able to set or change a plugin-owned value at the node where they see it. Use the Markdown deadline as
the first editing example. The host owns the view entry and its anchor, and the view plugin validates the value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 From a checkbox in the outline, a reader can set a missing deadline or change an existing one.
- [ ] #2 The saved value appears immediately and after reopening. The source and unrelated view data stay intact.
- [ ] #3 Canceling leaves the previous value intact. An invalid value shows the plugin’s reason and leaves disk intact.
- [ ] #4 Only writable stored properties can be edited here. Source-derived and computed properties are read-only.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
