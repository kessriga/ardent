---
id: TASK-8
title: Write structure files for Rust source files
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-20 19:15'
labels: []
milestone: m-1
dependencies:
  - TASK-7
references:
  - openspec/changes/task-8-rust-structure
  - >-
    backlog/decisions/decision-6 -
    Outline-queries-can-widen-a-node-over-annotation-captures.md
  - >-
    backlog/decisions/decision-7 -
    A-sync-reports-a-syntax-error-and-keeps-the-last-structure-file.md
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

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Record decision-6 (outline queries get an @annotation capture that widens the node range) and decision-7 (a sync reports a syntax error and keeps the last structure file).
2. OpenSpec change task-8-rust-structure: new spec rust-format; deltas for outline (annotation captures) and sync (syntax errors; the plugin-less format scenario moves from rust to html).
3. Engine: @annotation capture widens the range; ScanError::Syntax with the line and column of the first ERROR or MISSING node.
4. Built-in Rust plugin (tree-sitter-rust =0.24.2): items (mod, impl, trait, fn incl. signatures, struct, enum, union, const, static, type, macro_rules!, extern blocks) and members (fields, tuple fields, variants, associated items). Doc comments and attributes are annotations. Register it in sync.
5. Tests: engine unit tests, Rust plugin unit tests (spec example, kinds, labels, annotations), insta snapshots (inline modules, impls, trait impls, parse error), CLI integration (Rust sync, syntax error keeps old structure and exits 1).
6. README and specs; verification gate; archive the change.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design choices from the user (2026-11-20): a syntax error is a file error that keeps the last good structure file; doc comments and attributes join a node through a new @annotation capture; items and their members (fields, variants, associated items) are nodes, statements in function bodies are not. Left out: use declarations, extern crate, and item-level macro invocations.
<!-- SECTION:NOTES:END -->
