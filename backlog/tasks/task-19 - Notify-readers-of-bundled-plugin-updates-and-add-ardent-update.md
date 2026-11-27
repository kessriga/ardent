---
id: TASK-19
title: Notify readers of bundled plugin updates and add ardent update
status: To Do
assignee: []
created_date: '2026-11-27 12:41'
updated_date: '2026-11-27 12:41'
labels: []
dependencies: []
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
- [ ] #1 Running `ardent` names project plugins with available updates and points readers to `ardent update`.
- [ ] #2 The check detects a changed bundled component even when its manifest version stays the same.
- [ ] #3 `ardent update` replaces outdated project packages with complete copies bundled with the running executable.
- [ ] #4 The command preserves project configuration, third-party plugins, and the choice of which plugins to install.
- [ ] #5 After an update, the notice clears. Running `ardent update` again reports success and changes no files.
- [ ] #6 If an update fails, the command names the affected plugin and keeps its previous package usable.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
