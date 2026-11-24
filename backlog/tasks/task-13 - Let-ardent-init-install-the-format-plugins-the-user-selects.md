---
id: TASK-13
title: Let ardent init install the format plugins the user selects
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24 11:35'
updated_date: '2026-11-24 20:57'
labels: []
milestone: m-3
dependencies:
  - TASK-12
references:
  - openspec/specs/ardent-layout/spec.md
  - >-
    backlog/decisions/decision-8 -
    Run-format-plugins-as-WebAssembly-components.md
  - >-
    backlog/tasks/task-14 -
    Support-XDG-plugins-and-settings-with-user-or-project-compilation-caches.md
  - openspec/changes/archive/2026-11-24-task-13-select-plugins
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
- [x] #1 In a terminal, multi-select release plugins and install only those chosen in `.ardent/plugins/`.
- [x] #2 The generated `config.yaml` enables exactly the installed plugins under `formats`.
- [x] #3 A flag like `--plugins markdown,rust` skips prompts. Without a terminal or flag, init stops and names the flag.
- [x] #4 An unknown plugin name in the flag gives an error that lists the available plugins.
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
1. Specify terminal and scripted selection, package installation, and config generation in ardent-layout.
2. Implement selection and copy the chosen release files during init. Preserve existing project checks and rollback.
3. Exercise init and sync through the CLI, update the README, run the verification gate, and archive the change.

4. Address the review findings: recheck the project root after selection, handle short terminals and interruption, stage
   the CLI test release, restore the upgrade example, and verify both build profiles.

5. Fix executable and working directory resolution, install complete release packages, stop scripts when setup fails,
   support both arrow encodings and Windows pipe terminals, and stage test executables under Cargo. Verify these cases
   and both build profiles.

6. Restore terminal settings on external termination, require exact release filenames, and exclude the installation
   directory from the package walk. Cover all three paths with CLI tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
For installation scope, see decision-8, “Configuration and cache scope”. A project plugin takes precedence over a
same-named plugin in the user’s XDG configuration, and project settings override overlapping user settings. Installing
portable plugins and selecting a user or project compilation cache are separate concerns.

The picker uses rustix for Unix terminal reads and console for native Windows key input. Windows terminals with pipe
input accept a comma-separated list. The test fixture stages the release beside the Cargo executable so temporary
project directories can disallow execution.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Init installs and enables the selected release packages, including ancillary files. It resolves executable symlinks,
requires the exact spelling of package filenames, and skips its own output when the project is inside a package. The
picker handles split arrow sequences, restores the Unix terminal on termination, and refuses a working directory
replaced before confirmation. The README commands stop before sync when setup fails.

All 138 workspace tests passed in debug and release on macOS. The release binary restored terminal settings for four
signals during both an idle read and a partial arrow sequence. All workspace and test targets passed compilation checks
for Linux ARM64 and Windows x86-64.
<!-- SECTION:FINAL_SUMMARY:END -->
