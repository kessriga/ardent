---
id: TASK-2
title: Decide how anchors reference nodes and source ranges
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:28'
updated_date: '2026-11-19 21:24'
labels: []
milestone: m-0
dependencies:
  - TASK-1
documentation:
  - backlog/docs/doc-1 - Prior-art-outline-storage-and-node-addressing.md
priority: high
type: spike
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A view entry, such as a team comment on a function, must point at a node of a source file. The source file changes all the time, often outside Ardent. If the anchor is a byte offset, a one-line insertion above the function breaks every comment below it. If the anchor is a content hash, any edit to the function body orphans its comments. Candidates include byte or line ranges, tree paths (for example `mod foo / fn bar`), content hashes, stable IDs stored in `.ardent/`, and combinations with fallback matching. Every later milestone builds on this choice.

The README example "comments and backlinks" widens the question. View content can hold references such as `@foo` and `@bar` that a person types, and they point at nodes in other source files, including sections of a Markdown spec. Anchors and references should use one addressing scheme for the whole project. doc-1 (prior-art survey) is the starting point.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Backlog decision states the anchor representation and the rule for re-resolving an anchor after the source file changes
- [x] #2 The decision walks through at least these edits: insert lines above the node, edit the node body, rename the node, move the node to another parent, delete the node
- [x] #3 The decision states what happens to a view entry whose anchor cannot be resolved
- [x] #4 The decision states which parts of the anchor a format plugin must supply
- [x] #5 The decision defines a reference syntax that a person can type and read inside view content, and states how it resolves to a node
- [x] #6 The decision covers anchors and references to nodes in other source files, including sections of a Markdown document
- [x] #7 The decision states what a stored reference keeps when its target is renamed or moved: the typed text, a stable target, or both, and what the user sees afterwards
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [x] #2 Changed Markdown is formatted with rumdl
- [x] #3 README and specs match the new behavior
- [x] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Agree the design with the user in conversation (done: readable path identity, fingerprint fallback, three states, @[path] syntax, search prefixes).
2. Write the Backlog decision record covering AC #1-#7.
3. Read the record as an implementer would and close gaps.
4. Point the README open question at the decision.
5. Format Markdown with rumdl, run the cargo gate, open the PR.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design agreed with the user in conversation: readable node path as identity (no opaque IDs), content fingerprint as fallback, picker with optional search prefix for people, @[path] form for agents, rebind-and-mark-unconfirmed when unsure, orphans kept.
Backlog CLI 1.53.0 has no command to write a decision body, and 'decision create' failed until backlog/decisions/ existed. Created the folder, created decision-1 with the CLI, then filled its template sections by hand; frontmatter untouched.
Merging two nodes into one orphans the links of one old node, because the matcher maps one to one. This differs from the 'unconfirmed' outcome mentioned in discussion; recorded in the decision's Consequences.
DoD 3: README open question and glossary updated; openspec/specs is empty, no spec to change. DoD 4: spike, no OpenSpec change.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Recorded decision-1 (accepted): anchors and references address a node by its node path (file#label/label) plus a content fingerprint that excludes the label. Re-scans match old and new outlines and rewrite paths; a path that no longer resolves falls back to a project-wide fingerprint search. Links have three statuses (confirmed, unconfirmed with 'was', orphan); orphaned view entries are kept. References are written @[path]; the TUI picker accepts a search prefix such as @spec:foo. The decision lists what format plugins supply and what TASK-3, TASK-4, and TASK-5 must settle. README and AGENTS.md gained the terms node path and fingerprint. Verified: each acceptance criterion maps to a section of decision-1; rumdl check clean; cargo fmt/clippy/test pass (no code changed).
<!-- SECTION:FINAL_SUMMARY:END -->
