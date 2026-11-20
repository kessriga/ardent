# Design

## Context

This change writes specs only. decision-4 gives each layout and schema choice with its reasons, and the specs
`ardent-layout` and `ardent-schemas` state the requirements. This document covers how the M1 tasks should build against
them. The workspace has two crates, `ardent-core` and `ardent-cli`, and neither has code for `.ardent/` yet. decision-3
adds the crate `ardent-format` in M1.

## Goals / Non-Goals

**Goals:**

- Give TASK-6, TASK-7, and TASK-8 one home for each kind of type that the specs need.
- Name the constraints on the YAML library that the specs imply, before a task picks one.

**Non-Goals:**

- Picking the YAML library. The first task that reads or writes an Ardent file picks it.
- Matcher, plugin host, and backlink code. Later tasks build them.

## Decisions

### Crates

- `ardent-core` holds the types for every file in `.ardent/` (configuration, manifest, structure, view, derived view,
  backlink index), the fingerprint and content hash, and the mapping from a source path to its file paths. These are
  part of the document model and need no terminal, parser, or plugin runtime. `ardent-core` gains the `blake3` crate and
  a YAML crate.
- `ardent-format` turns a source file into an outline (decision-3). It calls `ardent-core` to compute fingerprints, so
  that one function computes them for every format.
- `ardent-cli` finds the project root and runs commands. `ardent init` lives here and writes the content that the spec
  `ardent-layout` gives under "Initial content".
- Locking and atomic writes touch the file system but no terminal. They live in `ardent-core`, next to the code that
  writes the files.

### Worked examples

The byte ranges, fingerprints, sizes, and hashes in the spec examples come from a throwaway generator. The generator ran
the example queries of decision-3 with `tree-sitter` 0.27.2, `tree-sitter-md` 0.5.4, and `tree-sitter-rust` 0.24.2, and
computed hashes with the `blake3` crate. The Python `blake3` package recomputed one fingerprint independently. The
generator stays outside `crates/`, as AGENTS.md requires. TASK-7 and TASK-8 can copy the example sources and expected
YAML into snapshot tests. `mtime` and `format_hash` are example values and need masks in such tests.

### Constraints on the YAML library

The specs need a YAML library that can:

- write keys in struct order, and plugin fields in the order the plugin returns them (an ordered map);
- write block style;
- reject unknown keys on read (`deny_unknown_fields` or equal) while it accepts any key inside `fields`;
- quote a text that a YAML 1.1 reader would read as another type, such as `no`, `on`, or `1e3`. People open these files
  with many tools, and some of them read YAML 1.1.

Ranges as `start..end` strings and timestamps as RFC 3339 strings need custom serialization, but no library feature.

## Risks / Trade-offs

- [The examples follow the draft queries of decision-3, not the real queries of TASK-7 and TASK-8] → The scenarios name
  "the example query of decision-3". TASK-7 and TASK-8 keep their own snapshot tests for the real queries.
- [An unquoted `mtime` reads as a timestamp type in YAML 1.1 tools] → Ardent reads it as a string. Other tools see an
  equal value.
- [Strict unknown-key checks reject files from a newer Ardent that added an optional key without a new schema version] →
  Every new key needs a new schema version. The specs require this, because an unknown key is an error.
