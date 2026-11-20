---
id: TASK-7
title: Write structure files for Markdown source files
status: To Do
assignee: []
created_date: '2026-11-19 16:29'
updated_date: '2026-11-20 12:10'
labels: []
milestone: m-1
dependencies:
  - TASK-3
  - TASK-5
  - TASK-6
priority: medium
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Markdown is the simplest structured format and the closest to what Roam and Tana show, so it is the first format to prove the document model end to end. This task adds the outline types to ardent-core and the first built-in format support, shaped after the format-plugin interface from TASK-3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ardent-core has outline and node types that match the structure schema from the .ardent/ spec
- [ ] #2 A command builds the outline of each tracked Markdown file and writes its structure file to .ardent/
- [ ] #3 Headings nest by level, and lists and paragraphs become child nodes of the heading above them
- [ ] #4 Each node records the source range the anchor decision requires
- [ ] #5 Round-trip tests show that writing a structure file and reading it back gives an equal outline
- [ ] #6 Snapshot tests cover a Markdown file with nested headings, lists, code blocks, and front matter
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @claude
created: 2026-11-20 12:10
---
Nested projects, open question from TASK-6. `ardent init` refuses to run inside an existing project, but it does not check the directories below: `ardent init` in a parent of an existing project still creates a nested project. Under the current "Tracked files" rule of `ardent-layout`, the outer project would then also track every file of the inner project, with other node paths and a separate `views/`. When this task builds the file scan, decide what the sync does with a directory that holds its own `.ardent/`, for example skip it as a separate project and report it. If the answer changes "Tracked files", update the spec in this task's OpenSpec change. Reuse `project_root::find` in ardent-cli for the root search.
---
<!-- COMMENTS:END -->
