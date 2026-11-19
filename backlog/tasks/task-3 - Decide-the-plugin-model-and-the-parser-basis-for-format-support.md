---
id: TASK-3
title: Decide the plugin model and the parser basis for format support
status: To Do
assignee: []
created_date: '2026-11-19 16:28'
updated_date: '2026-11-19 20:51'
labels: []
milestone: m-0
dependencies:
  - TASK-1
documentation:
  - backlog/docs/doc-1 - Prior-art-outline-storage-and-node-addressing.md
priority: high
type: spike
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Format support (Markdown, Rust, HTML) and views (complexity, comments) should come from plugins, so Ardent can grow without core changes. WebAssembly is the likely plugin format because the same plugins could run in a future web UI. Two linked questions are open: what a format plugin and a view plugin export, and whether format support builds on tree-sitter grammars or on a dedicated parser per format. M1 ships built-in Markdown and Rust support before the plugin host exists, so that built-in code must already follow the format-plugin interface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Backlog decision names the plugin runtime (for example wasmtime with the WebAssembly component model) or rejects WebAssembly with reasons
- [ ] #2 The decision defines the inputs and outputs of a format plugin and of a view plugin, at the level of a draft interface
- [ ] #3 The decision chooses tree-sitter grammars, per-format parsers, or a mix, and states the effect on anchors
- [ ] #4 The decision states how M1 built-in format support maps onto the format-plugin interface
- [ ] #5 The view-plugin interface treats references as values the host understands, not as private plugin data: a plugin can write references, and a plugin such as a backlink view can list the references that point at a node from any source file. TASK-2 defines the reference format itself
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
