---
id: TASK-19
title: Notify readers of bundled plugin updates and add ardent update
status: Done
assignee:
  - '@codex'
created_date: '2026-11-27 12:41'
updated_date: '2026-11-27 16:23'
labels: []
dependencies: []
references:
  - openspec/changes/archive/2026-11-27-task-19-plugin-updates
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Updating Ardent refreshes the plugins beside the executable. Existing projects keep their own copies in
`.ardent/plugins/`. A rebuilt plugin can still have the same version in `plugin.yaml`, so readers currently have to
compare and copy packages by hand. Show a notice in the browser, preferably on its status line, and let readers apply
the update when ready.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Running `ardent` names project plugins with available updates and points readers to `ardent update`.
- [x] #2 The check detects a changed bundled component even when its manifest version stays the same.
- [x] #3 `ardent update` replaces outdated project packages with complete copies bundled with the running executable.
- [x] #4 The command preserves project configuration, third-party plugins, and the choice of which plugins to install.
- [x] #5 After an update, the notice clears. Running `ardent update` again reports success and changes no files.
- [x] #6 If an update fails, the command names the affected plugin and keeps its previous package usable.
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
1. Share release discovery and compare installed package paths and bytes.
2. Add ardent update with staged verification, per-package rollback, and tests for failure and repeat runs.
3. Show the browser notice and clear it after package changes. Cover both entry points and document the command.
4. Run the project gate and archive the change in the implementation PR.

Resolve review findings for interruption, private staging, per-package results, read-only comparisons, cleanup and
deterministic failure and lock tests. Recheck the published PR.

Cover the second review cases with regression tests. Validate project configuration and installed manifests, restore
only proven updates without removing unrelated files, isolate recovery failures by format, preserve Unix package access,
and share release-root checks. Run the full gate and update PR 22.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Confirmed during review: running `ardent update` authorizes complete replacement of a bundled format plugin, including
local edits, extra files and a fork installed under the same plugin identity. The command requires no second prompt.
Separately named plugins keep their files.

The approved recovery directory under .ardent/plugins keeps the previous package across cache deletion. Sync, browse and
update recover under the project lock. Windows uses an owner-only access control list during staging and restores
project access on publication.

Portable recovery now verifies the replacement before retiring the previous package and preserves unrelated files. An
ambiguous package stays available for manual recovery while healthy formats sync. The macOS-only exacl dependency
preserves named access rules and their inheritance on shared installations.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The browser identifies available format plugin updates and reports comparison errors. The update command validates
project configuration and plugin names, preserves shared package access, and uses the same release-root checks as init.
After an interruption the next command can restore the original or verify the replacement. Ambiguous recovery keeps both
copies. On macOS the full gate passed 209 Rust tests and both recipe tests with two test threads. All eight specs passed
strict validation. The Linux and Windows CLI and test targets passed Clippy with warnings denied. Native runtime tests
ran on macOS.
<!-- SECTION:FINAL_SUMMARY:END -->
