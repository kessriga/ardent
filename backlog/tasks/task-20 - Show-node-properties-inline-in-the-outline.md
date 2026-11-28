---
id: TASK-20
title: Show node properties inline in the outline
status: Done
assignee:
  - '@codex'
created_date: '2026-11-28 12:11'
updated_date: '2026-11-28 16:37'
labels: []
dependencies: []
references:
  - openspec/specs/outline-browser/spec.md
  - openspec/specs/rust-format/spec.md
  - openspec/specs/markdown-format/spec.md
  - openspec/changes/archive/2026-11-28-task-20-inline-properties
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
- [x] #1 In the outline, Tab shows indented `⚬ name: value` rows directly below the selected node, before its children.
- [x] #2 A second Tab hides the rows. Both actions preserve selection, source preview and child expansion.
- [x] #3 For a `pub` Rust static of type `Gauge`, `static EAST_FIELD` shows `visibility: pub` and `type: Gauge`.
- [x] #4 Markdown checkbox nodes such as `item.1` show `checked: true` for `[x]` and `checked: false` for `[ ]`.
- [x] #5 Plain list items omit `checked`. Opening a node with no properties adds no blank rows.
- [x] #6 Shift+Tab switches panes both ways in wide and narrow terminals and preserves the reader’s place.
- [x] #7 The browser key hints and reader guide explain both bindings.
- [x] #8 Several nodes can show their properties at once. Only properties the plugin marks for inline display appear.
- [x] #9 Rust function generics, inputs and explicit returns have selectable nodes with their written properties.
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
Add the optional format property interface, derive Markdown and Rust properties and Rust signature nodes, load and
render them in the browser, then rebuild both release packages and run the verification gate.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Readers can keep properties open on several nodes. Markdown shows checkbox state. Rust shows applicable declaration
properties and selectable function signature children. Verified with just check, native guest and packaging tests,
terminal interaction tests, and an upgrade from the previous release packages on unchanged source.
<!-- SECTION:FINAL_SUMMARY:END -->
