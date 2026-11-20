---
id: TASK-10
title: Decide how plugins contribute dashboards
status: To Do
assignee: []
created_date: '2026-11-20 11:38'
labels: []
milestone: m-2
dependencies:
  - TASK-3
  - TASK-5
references:
  - >-
    backlog/decisions/decision-3 -
    Format-plugins-are-tree-sitter-grammars-with-outline-queries-and-view-plugins-are-WebAssembly-components.md
  - >-
    backlog/decisions/decision-4 -
    The-.ardent-directory-keeps-views-per-source-file-in-git-and-derived-data-in-a-rebuildable-cache.md
priority: medium
type: spike
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The left panel of Ardent shows a file explorer, but people also need dashboards built from views: recently added comments, tasks planned for today, GTD lists, or a kanban board of Backlog.md tasks. A discussion during TASK-5 concluded that a dashboard belongs to the view plugin that owns its data, because only that plugin knows what its fields mean. A separate dashboard plugin would depend on the field names of another plugin. Ardent keeps the drawing: a plugin returns a tree and the name of a layout, so the same plugins can run in a future web UI and never control the terminal. Generic dashboards that use only keys Ardent owns, such as the file explorer and recent activity across views, can be built in.

Two gaps remain. Stored entries have no creation time, and their random IDs carry none on purpose (decision-4), so "recent" has nothing to sort by. Ardent writes only inside `.ardent/`, so a board over source files such as `backlog/tasks/*.md` would be read-only. No Ardent writes view files before M2, so schema 1 can still gain keys without a migration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Backlog decision defines how a view plugin contributes dashboards: the interface it exports, its inputs (including the current time, because the host fixes the plugin clock), the host functions it may call, and where its settings live in `config.yaml`
- [ ] #2 The decision lists the layouts that Ardent draws, such as outline, board, and table, and the data a plugin returns for each
- [ ] #3 The decision names the dashboards built into Ardent, including the file explorer, and works through two examples: a kanban board of Backlog.md tasks from a backlog plugin, and a list of recently added comments
- [ ] #4 The decision states whether stored entries gain a `created` key that Ardent writes; if they do, an OpenSpec change updates the spec `ardent-schemas` and is archived in the same PR
- [ ] #5 The decision states whether a plugin may request an edit to a source file, such as moving a Backlog.md task to another status on a board, or that boards over source files are read-only
- [ ] #6 A throwaway check shows that Wasmtime can host one view plugin with the dashboards export and one without it, or the decision chooses another way to make the export optional
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
