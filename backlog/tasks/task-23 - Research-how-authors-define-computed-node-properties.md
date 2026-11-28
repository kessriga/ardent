---
id: TASK-23
title: Research how authors define computed node properties
status: To Do
assignee: []
created_date: '2026-11-28 12:12'
labels: []
dependencies: []
references:
  - >-
    backlog/decisions/decision-3 -
    Format-plugins-are-tree-sitter-grammars-with-outline-queries-and-view-plugins-are-WebAssembly-components.md
  - openspec/specs/ardent-schemas/spec.md
type: spike
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ardent already has a derived view plugin design. The open question is how an author defines a computed property, what
inputs it can read, and whether a dedicated expression language would help. Use the count of checked items beneath a
Markdown heading as a common example.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Compare a derived WebAssembly view plugin with at least one expression or query language. Cite primary sources.
- [ ] #2 Show how to define the same checkbox count in each and when the value updates after a checkbox changes.
- [ ] #3 Compare available inputs, missing values, evaluation errors and execution limits for each option.
- [ ] #4 Identify which parts of the derived view interface and cache each option can reuse and what it would change.
- [ ] #5 Record the trade-offs, recommendation and open questions in a Backlog decision for review.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
