---
id: TASK-12
title: Run the Markdown and Rust format plugins as WebAssembly components
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24 11:35'
updated_date: '2026-11-24 13:39'
labels: []
milestone: m-3
dependencies:
  - TASK-11
references:
  - openspec/specs/markdown-format/spec.md
  - openspec/specs/rust-format/spec.md
  - openspec/specs/sync/spec.md
  - openspec/changes/archive/2026-11-24-task-12-format-components
  - openspec/specs/format-components/spec.md
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
- [x] #1 Load only `.ardent/plugins/`. Ardent embeds no plugins and drops `tree-sitter-md` and `tree-sitter-rust`.
- [x] #2 Markdown and Rust components preserve node paths, ranges, and fingerprints for TASK-7 and TASK-8 snapshots.
- [x] #3 Warn for missing configured plugins. Report load errors and traps per plugin and keep syncing other formats.
- [x] #4 Per decision-7, report syntax error line and column, preserve the last structure file, and exit 1.
- [x] #5 Rust plugin docs describe the parser limit for Rust 2024 `safe` items in `unsafe extern` blocks.
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
1. Specify the project-only host and migration under decision-8. Use the existing Markdown and Rust snapshots as the
   compatibility baseline.
2. Package each grammar and outline query with the shared guest engine. Generate each manifest from its component.
3. Verify installed packages, validate scan results and integrate isolated instances into sync. Exercise cache
   invalidation, missing packages, syntax errors and trap recovery.
4. Document manual installation and the Rust parser limit, verify the CLI dependency tree and project gate, then archive
   the change with the implementation.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The CLI uses Wasmtime 49.0.7 with the repository Rust 1.96 toolchain. Both release packages match the existing outline
snapshots. CLI tests cover missing and broken packages, overlapping formats during failed verification, same-version
rebuilds, isolated traps, and malformed nodes. A clean project scanned 117 repository files, then zero on the next sync.
The Rust 2024 grammar limit reports line 1, column 25 for the documented declaration and keeps the prior structure.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Ardent runs project-installed Markdown and Rust WebAssembly components through Wasmtime. It verifies packages before
discovery, validates returned nodes, skips runtime startup for verified current files, and preserves the last good
structure on a failed scan. Both original snapshot suites passed unchanged. Verification passed for 104 host tests, 62
guest tests including a doctest, host and guest formatting and Clippy, and all seven main specs.
<!-- SECTION:FINAL_SUMMARY:END -->
