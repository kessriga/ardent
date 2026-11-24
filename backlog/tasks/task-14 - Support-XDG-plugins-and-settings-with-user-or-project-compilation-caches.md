---
id: TASK-14
title: Support XDG plugins and settings with user or project compilation caches
status: To Do
assignee: []
created_date: '2026-11-24 13:08'
updated_date: '2026-11-24 13:08'
labels: []
milestone: m-3
dependencies:
  - TASK-12
references:
  - >-
    backlog/decisions/decision-8 -
    Run-format-plugins-as-WebAssembly-components.md
  - openspec/specs/sync/spec.md
  - openspec/specs/ardent-layout/spec.md
priority: medium
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
People working across several projects should be able to install a format plugin once in their XDG configuration and
share settings across projects. Each project can pin its own plugin or override individual settings. The Wasmtime
compilation cache holds derived code and has a separate choice of scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Two projects can use the same enabled format plugin from the user’s XDG configuration.
- [ ] #2 A same-named project plugin wins. Removing it selects the user plugin. The stamp reflects the selected digest.
- [ ] #3 Project settings override matching XDG keys. Other user settings apply. User configuration is optional.
- [ ] #4 Either installation scope can use a per-user or per-project compilation cache and produces the same outline.
- [ ] #5 Sync may write to the selected cache. Its existing write boundary applies elsewhere.

- [ ] #6 Clearing either compilation cache preserves installed plugins and settings.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
