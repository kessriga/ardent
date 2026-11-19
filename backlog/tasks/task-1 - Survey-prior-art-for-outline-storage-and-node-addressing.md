---
id: TASK-1
title: Survey prior art for outline storage and node addressing
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:28'
updated_date: '2026-11-19 16:57'
labels: []
milestone: m-0
dependencies: []
documentation:
  - backlog/docs/doc-1 - Prior-art-outline-storage-and-node-addressing.md
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
- [x] #1 A Backlog doc records, for each surveyed tool, how it stores its tree, how it identifies a node, and how a reference to a node survives edits
- [x] #2 The doc covers at least Roam or Logseq, Tana, tree-sitter, one structural-diff tool, and Unison
- [x] #3 The doc ends with a short list of ideas to adopt and ideas to avoid, each linked to the spike it informs
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
1. Research each tool from primary sources (official docs, source code, specs): Roam, Logseq (file and DB graphs), Tana, tree-sitter, GumTree and difftastic, Unison.
2. Add annotation-anchoring prior art that bears directly on TASK-2: W3C Web Annotation selectors, Hypothesis fuzzy anchoring, code-review comment anchoring, and symbol monikers (SCIP/LSIF).
3. For each tool record: how it stores its tree, how it identifies a node, how a reference survives edits. Cite every claim.
4. End with ideas to adopt and to avoid, each linked to TASK-2, TASK-3, TASK-4, or TASK-5.
5. Format the draft with rumdl, then store it as a Backlog doc via backlog doc create and doc update.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Research ran from primary sources (docs, specs, source pinned to commit SHAs). Spot-checked three load-bearing claims at source: Hypothesis match-quote weights 50/20/20/2 and maxErrors min(256, len/2); tree-sitter Node::id reuse caveat; Gerrit ported comments "next best position". Claims without a primary source (Roam and Tana internals, GitHub outdated rule, SCIP stability) are marked as inference in the doc. DoD 3 and 4: no behavior change, so README, specs, and OpenSpec are untouched.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added doc-1, a prior-art survey of 11 subjects: Roam, Logseq, Tana, tree-sitter, GumTree, difftastic, Unison, W3C Web Annotation and Hypothesis, GitHub and Gerrit review comments, SCIP and LSIF, Org-mode and Obsidian. Main finding: no tool keeps references stable across outside edits without writing IDs into the source or owning the text. Ardent therefore needs Hypothesis-style redundant selectors, GumTree-style tree matching on resync, and an explicit orphan state. The doc ends with 15 adopt/avoid ideas, each linked to TASK-2 to TASK-5. Verified each acceptance criterion by scripted checks over the doc; cargo fmt, clippy, test, and rumdl pass.
<!-- SECTION:FINAL_SUMMARY:END -->
