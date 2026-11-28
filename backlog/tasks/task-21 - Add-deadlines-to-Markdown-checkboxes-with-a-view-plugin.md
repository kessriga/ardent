---
id: TASK-21
title: Add deadlines to Markdown checkboxes with a view plugin
status: To Do
assignee: []
created_date: '2026-11-28 12:11'
labels: []
dependencies:
  - TASK-20
references:
  - openspec/specs/ardent-layout/spec.md
  - openspec/specs/ardent-schemas/spec.md
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A team can attach a deadline to a Markdown checkbox while keeping the task text in Markdown. Use a separate stored view
plugin to exercise the same property display with a value that lives outside the source file. A fixture with a stored
deadline is enough to demonstrate the plugin before the editor exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An enabled deadlines view plugin shows a stored `deadline: 2026-12-01` alongside the checkbox’s `checked` row.
- [ ] #2 Only the checkbox named by the entry’s anchor shows that deadline. Plain list items have no deadline.
- [ ] #3 For `notes.md`, the plugin-owned deadline field lives in `.ardent/views/notes.md/deadlines.yaml`.
- [ ] #4 After a cache rebuild and reopen, the same checkbox shows its deadline and the Markdown source is unchanged.
- [ ] #5 With the deadlines view plugin absent, the checkbox shows `checked` and omits `deadline`.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
