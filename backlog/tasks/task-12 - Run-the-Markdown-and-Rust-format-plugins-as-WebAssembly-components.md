---
id: TASK-12
title: Run the Markdown and Rust format plugins as WebAssembly components
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24 11:35'
updated_date: '2026-11-24 17:01'
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

5. Fix scan initialization, rename handling, package identity, SIMD determinism, package input limits, diagnostic
   limits, and Rust documentation packaging. Verify regressions and update PR 13.

6. Preserve cache aliases before pruning. Validate disabled manifests and exact package filenames. Bound source reads
   and trap diagnostics. Fix space-safe release builds, document the WIT world, and expose the native scan error. Run
   regression, packaging, and workspace checks.

7. Preserve the last good structure for a mis-cased package, prune after complete discovery despite disabled manifest
   errors, reject directory-only component paths, and stage both release packages before updating the destination.
   Verify each regression and the workspace gates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The CLI uses Wasmtime 49.0.7 with the repository Rust 1.96 toolchain. Both release packages match the existing outline
snapshots. CLI tests cover missing and broken packages, overlapping formats during failed verification, same-version
rebuilds, isolated traps, and malformed nodes. A clean project scanned 117 repository files, then zero on the next sync.
The Rust 2024 grammar limit reports line 1, column 25 for the documented declaration and keeps the prior structure.

Fixed the seven PR review findings. Query preparation has its own bounded call. The host uses deterministic SIMD, bounds
package reads and guest diagnostics, and validates package directory spelling. Sync preserves current node paths and the
last good structure through a case-only rename during incomplete discovery. Staging includes the Rust parser notes.

Fixed all eight findings from the second review. Rename recovery and exact package spelling were exercised on
case-sensitive and case-insensitive APFS. Disabled manifests use the same schema and pattern rules as enabled packages
without starting the component. Source and diagnostic limits preserve the last good structure and let later files scan.
An independent guest compiled the documented WIT world and matched the public query scan error. Two clean builds from
the same paths containing spaces produced identical bytes for all three components. Moving the checkout can change Cargo
crate hashes.

Fixed the four findings from the third review. Case-sensitive and case-insensitive APFS tests preserved the last good
outline after a package case change. An invalid disabled manifest still reports an error and permits stale-file cleanup
when discovery completes. Enabled and disabled manifests reject component paths ending in a separator or terminal dot.
Packaging tests covered missing and malformed inputs, rollback after a failed replacement, and a clean retry. The
packaged files matched the shipped release byte for byte.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Ardent runs project-installed Markdown and Rust components through Wasmtime. The original outline snapshots pass. Sync
preserves the last good structure when a package fails and prunes stale structures after complete discovery. The release
packager stages both packages and restores previous files if a replacement fails. All 121 host tests, 64 guest tests,
two packaging tests, formatting, Clippy, the build, and seven OpenSpec checks passed.
<!-- SECTION:FINAL_SUMMARY:END -->
