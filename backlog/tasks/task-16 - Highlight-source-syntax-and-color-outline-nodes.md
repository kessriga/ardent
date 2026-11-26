---
id: TASK-16
title: Highlight source syntax and color outline nodes
status: Done
assignee:
  - '@codex'
created_date: '2026-11-26 13:13'
updated_date: '2026-11-26 14:59'
labels: []
dependencies: []
references:
  - openspec/changes/archive/2026-11-26-task-16-syntax-highlighting
type: enhancement
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Help readers distinguish syntax in the source preview and node kinds in the outline. Start with Markdown and Rust. Let
format plugins identify source ranges and their syntax categories so the terminal browser can style them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Rust highlights keywords, strings, and comments. Markdown highlights headings and code.
- [x] #2 Outline labels use colors for node kinds. Readers can still identify the selected row.
- [x] #3 Selecting, wrapping, and scrolling preserve the text and keep highlights on the correct characters.
- [x] #4 Plugins without highlighting show plain source text and a navigable outline.
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
1. Define and test the optional component highlighter, including scanner-only fallback.
2. Add Rust and Markdown syntax categories and rebuild the packaged plugins.
3. Carry verified ranges into the browser and test cell styles through selection, wrapping, paging and refresh.
4. Verify the packaged examples and the project gate, then archive the OpenSpec change in the implementation PR.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The packaged Rust and Markdown format plugins supply syntax spans to the browser. The outline colors labels by kind and
keeps its selection marker. Scanner-only plugins retain plain previews and keyboard navigation.

Packaged-component tests check syntax ranges and trap recovery. Terminal tests exercise the example packages, live
refresh, scanner-only navigation, and styled cells across wrapping, paging, invalid UTF-8 and combining marks. The Rust
workspace, native guest and package checks passed.
<!-- SECTION:FINAL_SUMMARY:END -->
