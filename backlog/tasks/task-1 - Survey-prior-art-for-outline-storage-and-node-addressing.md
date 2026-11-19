---
id: TASK-1
title: Survey prior art for outline storage and node addressing
status: To Do
assignee: []
created_date: '2026-11-19 16:28'
labels: []
milestone: m-0
dependencies: []
priority: high
type: spike
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Ardent derives outlines from source files instead of owning the content, and it attaches views to nodes. Several tools already solve parts of this: block storage and block references in Roam, Tana, and Logseq; incremental syntax trees in tree-sitter; structural diffs and node matching in difftastic, GumTree, and ast-grep; content-addressed code in Unison. The anchor, sync, layout, and plugin spikes should start from what these tools learned, not from a blank page.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Backlog doc records, for each surveyed tool, how it stores its tree, how it identifies a node, and how a reference to a node survives edits
- [ ] #2 The doc covers at least Roam or Logseq, Tana, tree-sitter, one structural-diff tool, and Unison
- [ ] #3 The doc ends with a short list of ideas to adopt and ideas to avoid, each linked to the spike it informs
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verification gate passes: cargo fmt --check / cargo clippy --all-targets -- -D warnings / cargo test
- [ ] #2 Changed Markdown is formatted with rumdl
- [ ] #3 README and specs match the new behavior
- [ ] #4 OpenSpec change archived in the same PR when the task has one
<!-- DOD:END -->
