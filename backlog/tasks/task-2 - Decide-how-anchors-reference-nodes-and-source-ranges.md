---
id: TASK-2
title: Decide how anchors reference nodes and source ranges
status: To Do
assignee: []
created_date: '2026-11-19 16:28'
labels: []
milestone: m-0
dependencies:
  - TASK-1
priority: high
type: spike
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A view entry, such as a team comment on a function, must point at a node of a source file. The source file changes all the time, often outside Ardent. If the anchor is a byte offset, a one-line insertion above the function breaks every comment below it. If the anchor is a content hash, any edit to the function body orphans its comments. Candidates include byte or line ranges, tree paths (for example `mod foo / fn bar`), content hashes, stable IDs stored in `.ardent/`, and combinations with fallback matching. Every later milestone builds on this choice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Backlog decision states the anchor representation and the rule for re-resolving an anchor after the source file changes
- [ ] #2 The decision walks through at least these edits: insert lines above the node, edit the node body, rename the node, move the node to another parent, delete the node
- [ ] #3 The decision states what happens to a view entry whose anchor cannot be resolved
- [ ] #4 The decision states which parts of the anchor a format plugin must supply
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
