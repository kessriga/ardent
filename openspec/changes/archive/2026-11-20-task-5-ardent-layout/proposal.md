# Proposal

## Why

`ardent init` creates `.ardent/`, and every later feature reads and writes files there. The M1 tasks (TASK-6 to TASK-9)
need a written format to build against. decision-4 fixes the layout and the schemas; this change turns that decision
into the first specs of the project. It implements Backlog TASK-5.

## What Changes

- Specify the layout of `.ardent/`: the project root, the split into user data in git and a derived `cache/` that git
  ignores, and how a source path maps to its structure file, its view files, and its derived view files.
- Specify the project configuration `config.yaml`: which format plugins and view plugins run, and which files Ardent
  tracks.
- Specify where plugins live and the fields of a plugin manifest.
- Specify the YAML schema of a structure file, a view file, a derived view file, and the backlink index, with a `schema`
  version in every file.
- Specify the fingerprint hash, entry IDs, the order of entries in a view file, and how entries follow their anchors
  into another view file.
- Give worked examples for one Markdown file and one Rust file.

Out of scope:

- Code. This change only writes specs. TASK-6, TASK-7, and TASK-8 implement them.
- How the cache keeps the old text of leaf nodes for the matcher. The task that builds the matcher decides it.
- Label ranges in the structure. The task that adds edits through the outline adds them.
- Migration of user files between schema versions. The first change that raises a version defines it.
- The rules for node paths, statuses, and re-resolution. decision-1 defines them; these specs use them.

## Capabilities

### New Capabilities

- `ardent-layout`: where Ardent keeps each file in `.ardent/`, what `ardent init` writes, the project configuration,
  which files Ardent tracks, where plugins live, and the lock and temporary files.
- `ardent-schemas`: the YAML schema of each kind of file in `.ardent/`: schema versions, shared rules, fingerprints,
  structure files, view files, derived view files, and the backlink index.

### Modified Capabilities

None. `openspec/specs/` is empty.

## Impact

- No code changes. `ardent-core` gains outline and file types in TASK-7, built against `ardent-schemas`.
- TASK-6 builds `ardent init` against `ardent-layout`.
- The fingerprint uses BLAKE3. The task that first computes fingerprints adds the `blake3` crate.
