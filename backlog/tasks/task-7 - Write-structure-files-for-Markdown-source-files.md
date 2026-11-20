---
id: TASK-7
title: Write structure files for Markdown source files
status: Done
assignee:
  - '@claude'
created_date: '2026-11-19 16:29'
updated_date: '2026-11-20 13:33'
labels: []
milestone: m-1
dependencies:
  - TASK-3
  - TASK-5
  - TASK-6
references:
  - openspec/changes/archive/2026-11-20-task-7-markdown-structure
  - openspec/specs/outline/spec.md
  - openspec/specs/markdown-format/spec.md
  - openspec/specs/sync/spec.md
  - openspec/specs/ardent-layout/spec.md
priority: medium
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Markdown is the simplest structured format and the closest to what Roam and Tana show, so it is the first format to prove the document model end to end. This task adds the outline types to ardent-core and the first built-in format support, shaped after the format-plugin interface from TASK-3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ardent-core has outline and node types that match the structure schema from the .ardent/ spec
- [x] #2 A command builds the outline of each tracked Markdown file and writes its structure file to .ardent/
- [x] #3 Headings nest by level, and lists and paragraphs become child nodes of the heading above them
- [x] #4 Each node records the source range the anchor decision requires
- [x] #5 Round-trip tests show that writing a structure file and reading it back gives an equal outline
- [x] #6 Snapshot tests cover a Markdown file with nested headings, lists, code blocks, and front matter
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
1. OpenSpec change task-7-markdown-structure: ardent-layout delta (sync command, nested projects in Tracked files), new capabilities outline-query and markdown-format. Record decision-5: outline queries may set a level, and the engine nests following siblings under a node with a lower level (amends decision-3).
2. ardent-core: harden layout::write_atomic (reject absolute and .. paths, create parent dirs, never create .ardent itself); add outline types with node paths and fingerprints, structure file types with YAML read/write (serde-saphyr, blake3, jiff), and config.yaml types.
3. New crate ardent-format (tree-sitter 0.27.2, tree-sitter-md 0.5.4): generic outline engine for @node/@label/kind/level queries, and the built-in markdown plugin (heading, paragraph, list, item, code, quote, table, front_matter; ATX and setext headings nest by level). Snapshot tests with insta.
4. ardent-cli: project_root::find returns I/O errors and keeps the physical working directory, as git and cargo do; tracked-file walk with the ignore crate, which skips and reports nested projects; ardent sync with lock, stamp check, re-scan, and removal of stale structure files. Integration tests.
5. README, specs, verification gate, archive the change, finalize the task.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented the OpenSpec change task-7-markdown-structure: 13/13 tasks done. ardent sync writes structure files for tracked Markdown files. New crate ardent-format holds the outline engine (query captures @node/@label, properties kind and level) and the built-in markdown plugin 0.1.0. decision-5 records the level property. The PR #7 review findings are fixed: write_atomic rejects absolute and .. paths and creates parents; project_root::find reports I/O errors; the root search keeps the physical working directory, as git and cargo do. Nested projects are skipped and reported. Gate: fmt, clippy, and 91 tests pass; openspec validate --strict passes. Manual run on a copy of this repo: 54 files synced in about 1 s; a second run scanned 0.

Acceptance evidence. AC1: structure::tests::writes_the_rust_example_of_the_spec matches the spec's Rust structure file byte for byte. AC2: tests/sync.rs (18 integration tests), and a manual run on a copy of this repo (54 files). AC3: markdown::tests (nesting, setext, lists, paragraphs) and three insta snapshots. AC4: every node has a range; the spec example ranges match in markdown::tests::structure_of_the_spec_example and tests/sync.rs. AC5: structure::tests::reads_back_what_it_writes and round_trip_keeps_quoted_labels_and_old_times. AC6: tests/markdown.rs snapshot of guide.md (front matter, nested headings, lists, code blocks). Gate after the archive: cargo fmt --check, clippy -D warnings, 91 tests, openspec validate --specs --strict (5 specs).
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @claude
created: 2026-11-20 12:10
---
Nested projects, open question from TASK-6. `ardent init` refuses to run inside an existing project, but it does not check the directories below: `ardent init` in a parent of an existing project still creates a nested project. Under the current "Tracked files" rule of `ardent-layout`, the outer project would then also track every file of the inner project, with other node paths and a separate `views/`. When this task builds the file scan, decide what the sync does with a directory that holds its own `.ardent/`, for example skip it as a separate project and report it. If the answer changes "Tracked files", update the spec in this task's OpenSpec change. Reuse `project_root::find` in ardent-cli for the root search.
---

author: @claude
created: 2026-11-20 12:49
---
Review findings from PR #7 (TASK-6). They matter once this task writes structure files and searches for the project root.

1. `layout::write_atomic` accepts any `relative_path`. An absolute path or a `..` component writes outside `.ardent/`. Structure and view paths come from source paths in a cloned repo, so treat them as untrusted: in `write_atomic` itself, reject absolute paths and every component that is not a normal name.
2. `write_atomic` creates only `cache/tmp/`, not the parent directory of the target. Writing `cache/files/src/engine.rs/structure.yaml` fails with NotFound unless the caller creates `cache/files/src/engine.rs/` first. Decide whether `write_atomic` creates parents, as "Directories on demand" requires them. It also creates `ardent_dir` itself through `create_dir_all` when a caller passes a wrong root.
3. `project_root::find` uses `Path::is_dir`, which treats a permission or I/O error as "no `.ardent/` here" and skips that directory. A command can then pick an outer root or no root. Return errors other than NotFound.
4. `env::current_dir` returns the path with symlinks resolved. Started in `/p/docs`, a symlink to `/data/docs`, the root search checks `/data` and its parents and never finds `/p/.ardent/`. Decide whether the search uses the logical path (`$PWD`).
---
<!-- COMMENTS:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added ardent sync, which writes .ardent/cache/files/<path>/structure.yaml for each tracked Markdown file, skips files whose stamp is current, and deletes stale structure files. New crate ardent-format holds a generic outline-query engine and the built-in markdown plugin 0.1.0; ardent-core holds the outline, structure, and config types. decision-5 adds the query property level, which nests ATX and setext headings. Nested projects are skipped and reported (comment #1). The PR #7 review findings are fixed (comment #2): write_atomic rejects paths outside .ardent/ and creates parents; the root search reports I/O errors and keeps the physical working directory. New specs: outline, markdown-format, sync; ardent-layout changed. Verified with cargo fmt, clippy -D warnings, 91 tests (unit, insta snapshots, CLI integration), openspec validate --strict, and a manual sync of this repository.
<!-- SECTION:FINAL_SUMMARY:END -->
