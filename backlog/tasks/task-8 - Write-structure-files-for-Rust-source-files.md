---
id: TASK-8
title: Write structure files for Rust source files
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-20 20:26'
labels: []
milestone: m-1
dependencies:
  - TASK-7
references:
  - openspec/changes/archive/2026-11-20-task-8-rust-structure
  - openspec/specs/rust-format/spec.md
  - openspec/specs/outline/spec.md
  - openspec/specs/sync/spec.md
  - >-
    backlog/decisions/decision-6 -
    Outline-queries-can-widen-a-node-over-annotation-captures.md
  - >-
    backlog/decisions/decision-7 -
    A-sync-reports-a-syntax-error-and-keeps-the-last-structure-file.md
priority: medium
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rust is the second built-in format. It tests whether the document model handles source code as well as prose: items nest inside modules and impl blocks, names are not unique across a file, and doc comments belong to the item below them. If the outline types from the Markdown task do not fit Rust, this task is where that shows up.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each tracked Rust file gets a structure file with modules, impl blocks, traits, functions, structs, enums, and constants as nodes
- [x] #2 Items nest inside their inline modules, impl blocks, and traits
- [x] #3 Doc comments and attributes are part of the node they apply to
- [x] #4 A file that does not parse produces a clear error for that file and does not stop the other files
- [x] #5 Snapshot tests cover a Rust file with inline modules, impl blocks, trait impls, and a parse error
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
1. Record decision-6 (outline queries get an @annotation capture that widens the node range) and decision-7 (a sync reports a syntax error and keeps the last structure file).
2. OpenSpec change task-8-rust-structure: new spec rust-format; deltas for outline (annotation captures) and sync (syntax errors; the plugin-less format scenario moves from rust to html).
3. Engine: @annotation capture widens the range; ScanError::Syntax with the line and column of the first ERROR or MISSING node.
4. Built-in Rust plugin (tree-sitter-rust =0.24.2): items (mod, impl, trait, fn incl. signatures, struct, enum, union, const, static, type, macro_rules!, extern blocks) and members (fields, tuple fields, variants, associated items). Doc comments and attributes are annotations. Register it in sync.
5. Tests: engine unit tests, Rust plugin unit tests (spec example, kinds, labels, annotations), insta snapshots (inline modules, impls, trait impls, parse error), CLI integration (Rust sync, syntax error keeps old structure and exits 1).
6. README and specs; verification gate; archive the change.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design choices from the user (2026-11-20): a syntax error is a file error that keeps the last good structure file; doc comments and attributes join a node through a new @annotation capture; items and their members (fields, variants, associated items) are nodes, statements in function bodies are not. Left out: use declarations, extern crate, and item-level macro invocations.

Acceptance evidence. AC1: rust::tests::item_kinds, item_labels, and tests/sync.rs rust_file_gets_a_structure_file; a sync of a copy of this repository wrote structure files for all 22 .rs files. AC2: rust::tests::function_in_an_inline_module, items_of_a_trait, inherent_impl_and_trait_impl_of_one_type, and the insta snapshots. AC3: rust::tests::documented_struct, block_doc_comment_and_blank_lines, documented_field_and_variant, attribute_on_a_tuple_field, edited_doc_comment_changes_the_fingerprint. AC4: tests/sync.rs syntax_error_in_one_file_does_not_stop_the_others and syntax_error_keeps_the_structure_of_the_last_scan; a manual run printed 'ardent: crates/ardent-core/src/yaml.rs: syntax error at line 108, column 1', kept the structure file, and exited 1. AC5: tests/rust.rs snapshots of modules.rs, types.rs, and an inline file with a syntax error. Gate: cargo fmt --check, clippy -D warnings, 147 tests, rumdl on changed Markdown, openspec validate --specs --strict (6 specs). Known limit: the reported position follows tree-sitter's error recovery and can be after the real mistake. Not filed (needs approval): statements in function bodies as nodes; ordinary comments inside a doc-comment run.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added the built-in rust format plugin 0.1.0 on tree-sitter-rust 0.24.2. Items (mod, impl, trait, fn, struct, enum, union, const, static, type, macro_rules!, extern) and their members (fields, tuple fields, enum variants, associated items) are nodes, labelled by keyword and name, such as fn run or impl Display for Engine. decision-6 adds the @annotation capture, so the doc comments and attributes above an item, field, or variant are part of its node. decision-7: a file with a syntax error gives a file error with its line and column, keeps the structure file of its last scan, and makes ardent sync exit 1. New spec rust-format; specs outline and sync changed. Verified with cargo fmt, clippy -D warnings, 147 tests (unit, insta snapshots, CLI integration), openspec validate --strict, and a sync of a copy of this repository (87 files, no errors; a second run scanned 0).
<!-- SECTION:FINAL_SUMMARY:END -->
