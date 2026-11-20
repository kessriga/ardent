---
id: TASK-6
title: Implement ardent init
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-20 12:07'
labels: []
milestone: m-1
dependencies:
  - TASK-5
references:
  - openspec/changes/archive/2026-11-20-task-6-ardent-init
  - openspec/specs/ardent-layout/spec.md
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
- [x] #1 Running ardent init in a directory without .ardent/ creates it with the layout and configuration file from the .ardent/ spec
- [x] #2 Running ardent init where .ardent/ already exists changes nothing and exits with a clear message
- [x] #3 ardent init never writes outside .ardent/
- [x] #4 Integration tests run the binary in a temporary directory and cover both cases
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
1. OpenSpec change task-6-ardent-init: delta to ardent-layout for init where .ardent exists, init inside a project, and no writes outside .ardent/.
2. ardent-core: layout module with the .ardent paths and write_atomic through .ardent/cache/tmp/ (tempfile + sync_all + persist).
3. ardent-cli: project_root search; init refuses inside a project (parent holds .ardent/), creates .ardent/ with create_dir, writes .gitignore and config.yaml with write_atomic, removes the empty cache/tmp/ and cache/, and removes .ardent/ again if a write fails. clap init subcommand; exit 1 on refusal.
4. Integration tests in ardent-cli/tests run the binary in a tempfile dir: new project (exact tree and bytes), existing .ardent/ dir, .ardent file, and project in a parent (tree unchanged, exit 1).
5. Update README, run the gate, archive the change, open the PR.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (design.md of the archived change): init exits 1 when .ardent exists (as cargo init and uv init do); init refuses to run inside an existing project, because Ardent has no workspaces and a nested project would split the views of the same files; init writes through .ardent/cache/tmp/ and then removes the empty cache/ so .ardent/ holds only the two files; a failed write removes .ardent/ again. Dependencies: clap (command parsing), tempfile (atomic write and test directories).
Validation: cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test (8 tests) pass; config.yaml from a real run matches the spec block byte for byte; mutation check: dropping the cache clean-up or the parent search each fails one integration test.
Not covered by a test: the removal of .ardent/ after a failed write. Not handled: init in a parent directory of an existing project; TASK-7 decides what the sync does with a nested .ardent/.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added ardent init. It creates .ardent/ with .gitignore (/cache/) and config.yaml from the ardent-layout spec, writes both through an atomic write in ardent-core (layout::write_atomic), and refuses with exit status 1 when .ardent exists in the working directory or a parent directory is already a project root. Verified with 4 integration tests that run the binary in temporary directories and compare full directory trees before and after, plus unit tests for the atomic write and the project root search; the full gate passes.
<!-- SECTION:FINAL_SUMMARY:END -->
