---
id: TASK-6
title: Implement ardent init
status: To Do
assignee: []
created_date: '2026-11-19 16:29'
labels: []
milestone: m-1
dependencies:
  - TASK-5
priority: medium
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ardent init` is the entry point to every other command: it marks a directory as an Ardent project and creates `.ardent/` with the layout from the TASK-5 spec. Later commands find the project root by looking for `.ardent/`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Running ardent init in a directory without .ardent/ creates it with the layout and configuration file from the .ardent/ spec
- [ ] #2 Running ardent init where .ardent/ already exists changes nothing and exits with a clear message
- [ ] #3 ardent init never writes outside .ardent/
- [ ] #4 Integration tests run the binary in a temporary directory and cover both cases
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
