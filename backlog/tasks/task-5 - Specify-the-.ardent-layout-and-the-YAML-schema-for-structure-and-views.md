---
id: TASK-5
title: Specify the .ardent/ layout and the YAML schema for structure and views
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-20 11:24'
labels: []
milestone: m-0
dependencies:
  - TASK-2
  - TASK-3
  - TASK-4
references:
  - openspec/changes/archive/2026-11-20-task-5-ardent-layout
  - openspec/specs/ardent-layout/spec.md
  - openspec/specs/ardent-schemas/spec.md
documentation:
  - backlog/docs/doc-1 - Prior-art-outline-storage-and-node-addressing.md
priority: high
type: spike
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ardent init` creates `.ardent/`, and every later feature reads and writes files there. The files must stay readable and diffable by a person, which is why YAML is the chosen format. This spike turns the anchor, sync, and plugin decisions into a concrete on-disk layout. The result is the first OpenSpec spec of the project, so that M1 implementation tasks build against a written format.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Backlog decision fixes the directory layout of .ardent/, including how a source file path maps to its structure file and view files
- [x] #2 The schema carries a format version so a later change can migrate old files
- [x] #3 The decision covers project-level configuration such as which files are tracked
- [x] #4 An OpenSpec change named task-5-<slug> adds a spec for the YAML schema of a structure file and of a view file, with one worked example each for a Markdown file and a Rust file, and is archived into openspec/specs/
- [x] #5 The layout states where the project-wide backlink index lives, and that Ardent derives it from views and can rebuild it
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
1. Agree the open choices with the user: view file grouping, moved entries, entry order, old leaf text for the matcher.
2. Generate worked examples (one Markdown, one Rust file) with a throwaway generator in the scratchpad: real byte ranges from the decision-3 queries and real BLAKE3 fingerprints.
3. Create decision-4 with the CLI and fill it: directory layout and path mapping, user part vs cache, schema versions, config (tracked files, formats, views), plugin location and manifest, fingerprint hash, entry IDs and order, entry moves, backlink index, lock, temp files.
4. Propose OpenSpec change task-5-ardent-layout with specs for the layout and for the YAML schemas (structure, view, derived view, backlink index), apply it, and archive it into openspec/specs/.
5. Update the README open question and the How-it-should-work section.
6. Run rumdl on changed Markdown and the cargo gate.
7. Finalize task-5 and open a PR from spike/5-ardent-layout.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Choices agreed with the user: (1) view files grouped per source file: .ardent/views/<source path>/<view>.yaml; (2) when an anchor moves to another source file, Ardent moves the entry to that file's view file; (3) each entry has a random ID; Ardent owns file placement and order, inserts entries at their ID's sorted place and never reorders, replacing decision-2's append rule; plugins own field meaning and display order; (4) the old leaf text that the matcher's similarity phase needs is left to the matcher task, since the cache can change without migration.

Created decision-4 with the CLI (accepted). Backlog CLI 1.53.0 still has no command to write a decision body, so the template sections were filled by hand with the frontmatter untouched, as for decision-1 to decision-3.

Proposed OpenSpec change task-5-ardent-layout: proposal, design, tasks, and two new capabilities, ardent-layout (21 requirements) and ardent-schemas (25 requirements). openspec validate --strict passes.

Worked examples come from a throwaway generator in the session scratchpad (tree-sitter 0.27.2, tree-sitter-md 0.5.4, tree-sitter-rust 0.24.2, blake3), running the example queries of decision-3. The Python blake3 package recomputed the fn run fingerprint independently (a33b27c3382d2785). CRLF and LF checkouts gave identical fingerprints. All 14 YAML examples parse with PyYAML; view-file sizes and hashes match the backlink index example. A scratch git 2.49.0 repo confirmed that two appends at the end of one YAML list conflict and two inserts in different places merge cleanly.

Acceptance criteria map:
- AC1: decision-4 'Directory layout'; ardent-layout requirements 'Structure file location', 'View file location', 'Derived view file location', 'Entries follow their anchors'.
- AC2: decision-4 'Schema versions'; ardent-schemas requirements 'Schema version', 'Unknown schema in user data', 'Unknown schema in the cache'. A change that raises a user-data version must define its migration.
- AC3: decision-4 'Project configuration'; ardent-layout requirements 'Initial content', 'Format plugins in the configuration', 'View plugins in the configuration', 'Patterns', 'Tracked files'.
- AC4: ardent-schemas requirements 'Structure nodes' and 'View file', each with a Markdown and a Rust scenario; archived with /opsx:archive in this PR.
- AC5: decision-4 'Backlink index'; ardent-layout requirements 'Backlink index location' and 'Cache can be deleted'; ardent-schemas requirement 'Backlink index'.

Applied and archived the change into openspec/changes/archive/2026-11-20-task-5-ardent-layout; main specs openspec/specs/ardent-layout/spec.md (21 requirements) and openspec/specs/ardent-schemas/spec.md (25 requirements) are the delta bodies byte for byte. README now describes the user-data and cache parts of .ardent/, lists decision-4 under Design decisions (M0 is settled), and defines view entry; view entry is in the term lists of AGENTS.md and openspec/config.yaml. Validation: openspec validate --specs --strict passes; the scratchpad check script passes (14 YAML blocks parse, example sources and generator output match the specs, view-file stamps match the index); cargo fmt --check, clippy -D warnings, and cargo test pass (no tests yet); rumdl is clean on every hand-written Markdown file. This task file is left as the CLI writes it, like TASK-2 to TASK-4.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Specified the .ardent/ layout and its YAML schemas. decision-4 (accepted) keeps stored views in git at .ardent/views/<source path>/<view>.yaml and derived data (structure files, derived views, the backlink index, lock, temp files) in .ardent/cache/, which git ignores and Ardent rebuilds; every file carries a schema version; config.yaml selects formats, views, and excludes. Entries have random IDs, sit in ID order, and move with their anchors. The OpenSpec change task-5-ardent-layout added the first two specs, ardent-layout and ardent-schemas, with Markdown and Rust worked examples whose ranges, fingerprints, and hashes come from a throwaway generator and were recomputed independently. Verified with openspec validate --strict, the example check script, the cargo gate, and rumdl.
<!-- SECTION:FINAL_SUMMARY:END -->
