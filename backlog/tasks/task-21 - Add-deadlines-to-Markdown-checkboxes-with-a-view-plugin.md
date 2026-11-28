---
id: TASK-21
title: Treat Markdown checkboxes as tasks with the markdown-task view plugin
status: To Do
assignee: []
created_date: '2026-11-28 12:11'
updated_date: '2026-11-28 12:33'
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
The `markdown-task` view plugin treats Markdown checkboxes as tasks. Its first feature is an optional `deadline`
property. The checkbox and its text live in Markdown. The plugin stores the deadline in the view entry anchored to that
checkbox. A fixture with a stored deadline demonstrates the feature before the property editor exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With `markdown-task` enabled, the checkbox shows its stored `deadline: 2026-12-01` beside `checked`.
- [ ] #2 Only the checkbox named by the entry’s anchor shows that deadline. Plain list items have no deadline.
- [ ] #3 For `notes.md`, `markdown-task` stores the `deadline` field in `.ardent/views/notes.md/markdown-task.yaml`.
- [ ] #4 After a cache rebuild and reopen, the same checkbox shows its deadline and the Markdown source is unchanged.
- [ ] #5 With `markdown-task` absent, the checkbox shows `checked` and omits `deadline`.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
