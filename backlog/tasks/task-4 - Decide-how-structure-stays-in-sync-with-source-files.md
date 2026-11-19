---
id: TASK-4
title: Decide how structure stays in sync with source files
status: To Do
assignee: []
created_date: '2026-11-19 16:29'
labels: []
milestone: m-0
dependencies:
  - TASK-2
priority: high
type: spike
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
People and agents edit source files in their editors, with git, and with formatters, without Ardent running. The structure in `.ardent/` then goes stale, and views may point at nodes that moved. Ardent needs a rule for when it notices a change (on demand, file watcher, git hook) and how it rebuilds the structure while keeping anchors valid. A second question is whether `.ardent/` is committed to git, and if so, how merges of structure files work. Editing source files through the outline is a later milestone, but this decision must not rule it out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Backlog decision states how Ardent detects that a source file changed and when it rebuilds the structure
- [ ] #2 The decision states which side is authoritative when the source file and the structure disagree
- [ ] #3 The decision states whether .ardent/ is meant to be committed to git, and which files in it are derived and which are user data
- [ ] #4 The decision lists what a later edit-through-the-outline feature would need from this model
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
