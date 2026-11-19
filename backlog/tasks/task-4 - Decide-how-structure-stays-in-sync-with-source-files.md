---
id: TASK-4
title: Decide how structure stays in sync with source files
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-19 21:42'
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
- [x] #1 A Backlog decision states how Ardent detects that a source file changed and when it rebuilds the structure
- [x] #2 The decision states which side is authoritative when the source file and the structure disagree
- [x] #3 The decision states whether .ardent/ is meant to be committed to git, and which files in it are derived and which are user data
- [x] #4 The decision lists what a later edit-through-the-outline feature would need from this model
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
1. Agree the open choices with the user: git policy for .ardent/, how changes are detected, whether any run may rewrite view files.
2. Create decision-2 with the CLI and fill its sections: detection and re-scan rule, authority, matcher, commit policy (derived vs user data), what edit-through-the-outline needs.
3. Update the README open question to point at decision-2.
4. Run rumdl on changed Markdown and the cargo gate.
5. Finalize task-4 (ACs, summary, Done) and open a PR from spike/4-structure-sync.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design agreed with the user: check stamps on every run (no hooks, no daemon); every sync persists structure and link rewrites together; views and config are committed, while structure, stamps, backlink index, and lock are derived and ignored.

Backlog CLI 1.53.0 has no command to write a decision body. Created decision-2 with the CLI and filled its template sections by hand; frontmatter untouched, as for decision-1.

Matcher: GumTree-style exact phase (path, unique fingerprint), then a similarity phase. The cinereus crate is named as a candidate to evaluate against the edit table of decision-1. DoD 3: README open question and glossary (structure, sync) updated; AGENTS.md term list gained sync; openspec/specs is empty. DoD 4: spike, no OpenSpec change.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Recorded decision-2 (accepted). Every Ardent command starts with a sync: it compares each tracked file's stamp (plugin name and version, size, mtime, content hash), re-scans changed files, maps old to new outlines with a GumTree-style two-phase matcher, re-resolves links by decision-1, and writes structure and link rewrites together. The source file is authoritative; the structure is a rebuildable local cache. Views and project configuration are committed; structure, stamps, backlink index, and lock are derived and ignored. The decision lists six needs for a later edit-through-the-outline feature and follow-ups for TASK-3 and TASK-5. Verified: each acceptance criterion maps to a section of decision-2; rumdl check clean; cargo fmt --check, clippy -D warnings, and cargo test pass (no code changed).
<!-- SECTION:FINAL_SUMMARY:END -->
