---
id: TASK-3
title: Decide the plugin model and the parser basis for format support
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:28'
updated_date: '2026-11-20 00:14'
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
- [x] #1 A Backlog decision names the plugin runtime (for example wasmtime with the WebAssembly component model) or rejects WebAssembly with reasons
- [x] #2 The decision defines the inputs and outputs of a format plugin and of a view plugin, at the level of a draft interface
- [x] #3 The decision chooses tree-sitter grammars, per-format parsers, or a mix, and states the effect on anchors
- [x] #4 The decision states how M1 built-in format support maps onto the format-plugin interface
- [x] #5 The view-plugin interface treats references as values the host understands, not as private plugin data: a plugin can write references, and a plugin such as a backlink view can list the references that point at a node from any source file. TASK-2 defines the reference format itself
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
1. Research runtimes (wasmtime component model, Extism), parser options (tree-sitter, pulldown-cmark, ra_ap_syntax), and prior art (Zed extensions).
2. Present the runtime, parser-basis, and view-plugin choices to the user and get decisions.
3. Write decision-3 with a draft WIT interface for format and view plugins, the effect on anchors, and the M1 mapping.
4. Update README open questions, close TASK-3, open a PR.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User chose: Wasmtime + component model; tree-sitter for every format (grammar + outline query, no code); every view is plugin code with its own fields.
Throwaway check in the session scratchpad (not committed): wit-parser 0.263.0 parsed the draft WIT; tree-sitter 0.27.2 + tree-sitter-rust 0.24.2 + tree-sitter-md 0.5.4 ran the example queries and gave mod engine/impl Engine/fn run and Spec/Sync rules/Renames; a half-typed fn helper( dropped only that function.
Facts: tree-sitter 0.27.2 wasm feature pins wasmtime-c-api-impl 48.0.8 (wasmtime latest 50.0.3); tree-sitter-md makes sections only for ATX headings.
backlog decision create has no body option, so the decision body was written into the CLI-created file; frontmatter is CLI-generated.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Recorded decision-3. Format plugins are data: a tree-sitter grammar (.wasm), an outline query (@node, @label, #set! kind), and a manifest; built-ins in M1 link tree-sitter-md and tree-sitter-rust natively through the same query engine in a new ardent-format crate. View plugins are Wasmtime components in a stored-view or derived-view world (draft WIT ardent:plugin@0.1.0) with flat plugin fields; Ardent finds @[…] references in text fields, and plugins read backlinks through a host function. Plugins get no files, network, or environment. Effects on anchors: label ranges come from @label captures, tree-sitter error recovery keeps sibling nodes, and Ardent's Wasmtime version follows tree-sitter's pin. README plugin text, open questions, and glossary updated. Verified: draft WIT parses with wit-parser; example queries give the expected node paths; cargo fmt/clippy/test pass; rumdl clean.
<!-- SECTION:FINAL_SUMMARY:END -->
