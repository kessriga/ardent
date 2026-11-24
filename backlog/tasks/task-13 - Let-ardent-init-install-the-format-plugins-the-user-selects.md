---
id: TASK-13
title: Let ardent init install the format plugins the user selects
status: To Do
assignee: []
created_date: '2026-11-24 11:35'
labels: []
milestone: m-3
dependencies:
  - TASK-12
references:
  - openspec/specs/ardent-layout/spec.md
priority: medium
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user wants the choice that `openspec init` gives for agents: after `ardent init`, a project has only the plugins the
user picked. The Markdown and Rust plugins ship as files with the Ardent release, but Ardent runs only the plugins
installed in `.ardent/plugins/` (TASK-12). Where third-party plugins come from is out of scope for now.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In a terminal, multi-select release plugins and install only those chosen in `.ardent/plugins/`.
- [ ] #2 The generated `config.yaml` enables exactly the installed plugins under `formats`.
- [ ] #3 A flag like `--plugins markdown,rust` skips prompts. Without a terminal or flag, init stops and names the flag.
- [ ] #4 An unknown plugin name in the flag gives an error that lists the available plugins.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
