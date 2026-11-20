---
id: TASK-6
title: Implement ardent init
status: In Progress
assignee:
  - '@claude'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-20 12:00'
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

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. OpenSpec change task-6-ardent-init: delta to ardent-layout for init where .ardent exists, init inside a project, and no writes outside .ardent/.
2. ardent-core: layout module with the .ardent paths and write_atomic through .ardent/cache/tmp/ (tempfile + sync_all + persist).
3. ardent-cli: project_root search; init refuses inside a project (parent holds .ardent/), creates .ardent/ with create_dir, writes .gitignore and config.yaml with write_atomic, removes the empty cache/tmp/ and cache/, and removes .ardent/ again if a write fails. clap init subcommand; exit 1 on refusal.
4. Integration tests in ardent-cli/tests run the binary in a tempfile dir: new project (exact tree and bytes), existing .ardent/ dir, .ardent file, and project in a parent (tree unchanged, exit 1).
5. Update README, run the gate, archive the change, open the PR.
<!-- SECTION:PLAN:END -->
