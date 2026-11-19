---
id: TASK-5
title: Specify the .ardent/ layout and the YAML schema for structure and views
status: To Do
assignee: []
created_date: '2026-11-19 16:29'
updated_date: '2026-11-19 20:51'
labels: []
milestone: m-0
dependencies:
  - TASK-2
  - TASK-3
  - TASK-4
documentation:
  - backlog/docs/doc-1 - Prior-art-outline-storage-and-node-addressing.md
priority: high
type: spike
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ardent init` creates `.ardent/`, and every later feature reads and writes files there. The files must stay readable and diffable by a person, which is why YAML is the chosen format. This spike turns the anchor, sync, and plugin decisions into a concrete on-disk layout. The result is the first OpenSpec spec of the project, so that M1 implementation tasks build against a written format.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Backlog decision fixes the directory layout of .ardent/, including how a source file path maps to its structure file and view files
- [ ] #2 The schema carries a format version so a later change can migrate old files
- [ ] #3 The decision covers project-level configuration such as which files are tracked
- [ ] #4 An OpenSpec change named task-5-<slug> adds a spec for the YAML schema of a structure file and of a view file, with one worked example each for a Markdown file and a Rust file, and is archived into openspec/specs/
- [ ] #5 The layout states where the project-wide backlink index lives, and that Ardent derives it from views and can rebuild it
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
