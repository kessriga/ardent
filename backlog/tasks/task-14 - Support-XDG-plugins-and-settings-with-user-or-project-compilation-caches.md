---
id: TASK-14
title: Support XDG plugins and settings with user or project compilation caches
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24 13:08'
updated_date: '2026-11-27 17:23'
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
  - openspec/changes/archive/2026-11-27-task-14-xdg-plugins
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
- [x] #1 Two projects can use the same enabled format plugin from the user’s XDG configuration.
- [x] #2 A same-named project plugin wins. Removing it selects the user plugin. The stamp reflects the selected digest.
- [x] #3 Project settings override matching XDG keys. Other user settings apply. User configuration is optional.
- [x] #4 Either installation scope can use a per-user or per-project compilation cache and produces the same outline.
- [x] #5 Sync may write to the selected cache. Its existing write boundary applies elsewhere.

- [x] #6 Clearing either compilation cache preserves installed plugins and settings.
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
1. Specify how optional user settings, project overrides and package selection interact with init and update.
2. Resolve settings before discovery. Verify and stamp the selected format plugin in sync and browse.
3. Select a trusted Wasmtime cache independently of package location. Test both installation scopes against both cache
   scopes, including clearing the cache and preserving inputs.
4. Update the README, run the workspace gate and archive the approved change.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Projects select their enabled formats and inherit user settings by key. Sync and browse verify the project pin or the
shared XDG package and use the selected compilation cache. The CLI tests cover two projects, the four installation and
cache combinations, cache clearing, digest changes after removing a pin, and a broken pin. The browser test reads
highlighted Rust source through a user installation. All 217 workspace tests, formatting and Clippy passed on macOS.
<!-- SECTION:FINAL_SUMMARY:END -->
