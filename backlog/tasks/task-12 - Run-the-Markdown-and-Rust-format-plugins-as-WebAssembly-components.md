---
id: TASK-12
title: Run the Markdown and Rust format plugins as WebAssembly components
status: To Do
assignee: []
created_date: '2026-11-24 11:35'
labels: []
milestone: m-3
dependencies:
  - TASK-11
references:
  - openspec/specs/markdown-format/spec.md
  - openspec/specs/rust-format/spec.md
  - openspec/specs/sync/spec.md
priority: medium
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ardent links the Markdown and Rust grammars and outline queries into its binary (TASK-7, TASK-8). The user wants no
built-in plugins inside Ardent: every plugin is a WebAssembly component that the project installs in `.ardent/plugins/`.
This task builds the plugin host and moves both plugins out of the binary, following the decision of TASK-11.

Two findings from the review of PR #9 belong here. First, tree-sitter-rust 0.24.2 rejects the valid Rust 2024 item
`unsafe extern "C" { pub safe fn f(); }`, so sync reports a syntax error that the user cannot fix. Second, the test
helper `syntax_error` in `crates/ardent-format/src/engine.rs` hides the production function of the same name; rename it
if the code survives the move.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Load only `.ardent/plugins/`. Ardent embeds no plugins and drops `tree-sitter-md` and `tree-sitter-rust`.
- [ ] #2 Markdown and Rust components preserve node paths, ranges, and fingerprints for TASK-7 and TASK-8 snapshots.
- [ ] #3 Warn for missing configured plugins. Report load errors and traps per plugin and keep syncing other formats.
- [ ] #4 Per decision-7, report syntax error line and column, preserve the last structure file, and exit 1.
- [ ] #5 Rust plugin docs describe the parser limit for Rust 2024 `safe` items in `unsafe extern` blocks.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
