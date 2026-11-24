---
id: TASK-11
title: Decide how format plugins run as WebAssembly components
status: To Do
assignee: []
created_date: '2026-11-24 11:34'
labels: []
milestone: m-3
dependencies: []
references:
  - >-
    backlog/decisions/decision-3 -
    Format-plugins-are-tree-sitter-grammars-with-outline-queries-and-view-plugins-are-WebAssembly-components.md
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
- [ ] #1 Replace decision-3’s format and M1 built-in sections. List rules retained from decisions 3, 5, 6, and 7.
- [ ] #2 Specify the WIT world with name, version, default patterns, and scan results (nodes or error line and column).
- [ ] #3 State where the outline engine runs and what authors supply for formats a query can express.
- [ ] #4 Outside `crates/`, use Wasmtime to scan a file with tree-sitter and one grammar in a `wasm32-wasip2` component.
- [ ] #5 Time release Rust plugin load and scan with and without Wasmtime’s compile cache. Specify how sync starts fast.
- [ ] #6 Specify the files in `.ardent/plugins/<name>/` and what replaces the format hash in the decision-2 stamp.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
