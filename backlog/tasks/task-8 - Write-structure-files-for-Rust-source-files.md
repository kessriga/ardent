---
id: TASK-8
title: Write structure files for Rust source files
status: To Do
assignee: []
created_date: '2026-11-19 16:29'
labels: []
milestone: m-1
dependencies:
  - TASK-7
priority: medium
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rust is the second built-in format. It tests whether the document model handles source code as well as prose: items nest inside modules and impl blocks, names are not unique across a file, and doc comments belong to the item below them. If the outline types from the Markdown task do not fit Rust, this task is where that shows up.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each tracked Rust file gets a structure file with modules, impl blocks, traits, functions, structs, enums, and constants as nodes
- [ ] #2 Items nest inside their inline modules, impl blocks, and traits
- [ ] #3 Doc comments and attributes are part of the node they apply to
- [ ] #4 A file that does not parse produces a clear error for that file and does not stop the other files
- [ ] #5 Snapshot tests cover a Rust file with inline modules, impl blocks, trait impls, and a parse error
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
