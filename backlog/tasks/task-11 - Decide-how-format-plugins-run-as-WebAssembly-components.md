---
id: TASK-11
title: Decide how format plugins run as WebAssembly components
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24 11:34'
updated_date: '2026-11-24 12:08'
labels: []
milestone: m-3
dependencies: []
references:
  - >-
    backlog/decisions/decision-3 -
    Format-plugins-are-tree-sitter-grammars-with-outline-queries-and-view-plugins-are-WebAssembly-components.md
  - >-
    backlog/decisions/decision-8 -
    Run-format-plugins-as-WebAssembly-components.md
documentation:
  - backlog/docs/doc-2 - Rust-format-component-measurements.md
priority: medium
type: spike
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-3 made a format plugin a data package (`plugin.yaml`, `grammar.wasm`, `outline.scm`) and let M1 link the
Markdown and Rust grammars natively into the Ardent binary. TASK-7 and TASK-8 built those two plugins that way. On
2026-11-21 the user changed the model: Ardent ships no built-in plugins, and every plugin, a format plugin included, is
a WebAssembly component with code. `ardent init` lets the user pick plugins from files that ship with the release, the
way `openspec init` lets the user pick agents.

The Markdown and Rust plugins must move out of the binary. Their outline queries, the engine rules of decision-5 and
decision-6, and the syntax-error rule of decision-7 should carry over. This spike settles the component design before
that migration. A review of PR #9 found that the Rust query takes about 0.08 s to compile on every sync; plugin start-up
time is part of this question.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace decision-3’s format and M1 built-in sections. List rules retained from decisions 3, 5, 6, and 7.
- [x] #2 Specify the WIT world with name, version, default patterns, and scan results (nodes or error line and column).
- [x] #3 State where the outline engine runs and what authors supply for formats a query can express.
- [x] #4 Outside `crates/`, use Wasmtime to scan a file with tree-sitter and one grammar in a `wasm32-wasip2` component.
- [x] #5 Time release Rust plugin load and scan with and without Wasmtime’s compile cache. Specify how sync starts fast.
- [x] #6 Specify the files in `.ardent/plugins/<name>/` and what replaces the format hash in the decision-2 stamp.
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
1. Trace the current scan contract and stamp against decisions 2, 3, 5, 6, and 7.
2. Build an isolated wasm32-wasip2 Rust format plugin outside crates/ and exercise it with Wasmtime.
3. Measure release startup and repeated scans with the compile cache disabled, empty, and warm.
4. Record the evidence and submit the component interface, installed files, and stamp rule for owner approval.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner approved the component interface, manifest and stamp rule. Decision-8 records the design. Doc-2 records the
release measurements and reproduction steps.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Verified the existing Rust engine inside a wasm32-wasip2 component. It returned 11 nodes and reproduced the syntax-error
position at line 6, column 5. Warm caching reduced median load from 187.57 to 17.45 ms. Decision-8 specifies lazy
loading and one instance per needed format per sync. All 147 workspace tests passed, along with formatting, Clippy,
build, and strict validation of six specs. This design spike has no OpenSpec implementation change to archive.
<!-- SECTION:FINAL_SUMMARY:END -->
