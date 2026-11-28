---
id: TASK-20
title: Show node properties inline in the outline
status: To Do
assignee: []
created_date: '2026-11-28 12:11'
labels: []
dependencies: []
references:
  - openspec/specs/outline-browser/spec.md
  - openspec/specs/rust-format/spec.md
  - openspec/specs/markdown-format/spec.md
priority: high
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Readers need to inspect a node while keeping its source and surrounding outline in view. A property is a named value
shown beneath its node, like a field in Tana. Source files supply the first examples. A separate view plugin will supply
stored properties using the same display.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In the outline, Tab shows indented `⚬ name: value` rows directly below the selected node, before its children.
- [ ] #2 A second Tab hides the rows. Both actions preserve selection, source preview and child expansion.
- [ ] #3 For a `pub` Rust static of type `Gauge`, `static EAST_FIELD` shows `visibility: pub` and `type: Gauge`.
- [ ] #4 Markdown checkbox nodes such as `item.1` show `checked: true` for `[x]` and `checked: false` for `[ ]`.
- [ ] #5 Plain list items omit `checked`. Opening a node with no properties adds no blank rows.
- [ ] #6 Shift+Tab switches panes both ways in wide and narrow terminals and preserves the reader’s place.
- [ ] #7 The browser key hints and reader guide explain both bindings.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
