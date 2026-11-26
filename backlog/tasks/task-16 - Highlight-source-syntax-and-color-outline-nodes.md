---
id: TASK-16
title: Highlight source syntax and color outline nodes
status: To Do
assignee: []
created_date: '2026-11-26 13:13'
updated_date: '2026-11-26 13:13'
labels: []
dependencies: []
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
- [ ] #1 Rust highlights keywords, strings, and comments. Markdown highlights headings and code.
- [ ] #2 Outline labels use colors for node kinds. Readers can still identify the selected row.
- [ ] #3 Selecting, wrapping, and scrolling preserve the text and keep highlights on the correct characters.
- [ ] #4 Plugins without highlighting show plain source text and a navigable outline.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
