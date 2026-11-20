# Tasks

Each task ends with a passing `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.

## 1. Decision and review fixes

- [x] 1.1 Record decision-5 with `backlog decision create`: outline queries may set `level`, and the engine nests
      following siblings under a node with a smaller level; it amends the query table of decision-3. Verify that
      `backlog decision list` shows it as accepted
- [x] 1.2 Harden `layout::write_atomic` in `ardent-core`: reject absolute paths and components other than normal names,
      fail when `ardent_dir` is not a directory, and create the parent directories of the target; verify with unit tests
      for `..`, an absolute path, a missing `ardent_dir` that stays missing, and a write to
      `cache/files/src/engine.rs/structure.yaml`
- [x] 1.3 Change `project_root::find` in `ardent-cli` to return `Result<Option<PathBuf>, (PathBuf, io::Error)>` and make
      `ardent init` report its errors; verify with a unit test for a directory that denies search permission, and with
      the existing init tests

## 2. Outline and structure types in ardent-core

- [x] 2.1 Add the module `outline`: the node data of a format plugin, the outline, node paths with escapes and `[n]`
      suffixes, and fingerprints; verify with unit tests for the scenarios "Node path", "Positional suffixes", and "Root
      node" of the spec `outline`, and the fingerprint scenarios of `ardent-schemas`
- [x] 2.2 Add `serde`, `serde-saphyr`, `jiff`, and `blake3`, and the modules `yaml` and `structure`: the stamp and the
      structure file with YAML read and write; verify with a round-trip test (write, read, compare equal), a test that
      the Rust example of `ardent-schemas` serializes to the spec text byte for byte, and tests that an unknown schema
      and an unknown key fail to read
- [x] 2.3 Add the module `config`: read `config.yaml` with the `schema` check, unknown keys, and plugin name checks;
      verify with unit tests for the initial configuration, a missing `schema`, `schema: 2`, the key `exlude`, and a
      view named `structure`

## 3. Outline engine and Markdown plugin in ardent-format

- [ ] 3.1 Create the crate `ardent-format` with `tree-sitter` 0.27.2 and `tree-sitter-md` pinned to `=0.5.4`, and the
      module `engine`: query compilation with `kind` and `level`, candidate nodes, containment, nesting by level, and
      labels; verify with unit tests for the scenarios of the spec `outline` that use a query, including "Invalid level"
- [ ] 3.2 Add the module `markdown` with `outline.scm`, the plugin constant, and `format_hash`; verify that the spec
      example of `ardent-schemas` gives exactly the paths, kinds, ranges, and fingerprints of the spec, and that unit
      tests cover each scenario of the spec `markdown-format`
- [ ] 3.3 Add `insta` snapshot tests for Markdown files with nested headings, setext headings, lists, code blocks, front
      matter, quotes, and tables; verify that `cargo test -p ardent-format` passes with the reviewed snapshots

## 4. ardent sync in ardent-cli

- [ ] 4.1 Add `ignore` and the module `tracked`: the walk with the settings of design.md, format and `exclude` patterns,
      and warnings for two formats, non-UTF-8 paths, and nested projects; verify with unit tests for each scenario of
      "Tracked files" in `ardent-layout`
- [ ] 4.2 Add the module `sync` and the subcommand `ardent sync`: configuration, plugins, lock, stamp check, scan,
      write, stale structure files, summary, and exit status; verify with integration tests for each scenario of the
      spec `sync` that a test can set up, and for "No project" and "Nested project" of `ardent-layout`
- [ ] 4.3 Update `README.md`: status, the `ardent sync` command, the Markdown outline, and the crate list; verify with
      `rumdl fmt --no-cache --enable MD013 README.md`

## 5. Checks

- [ ] 5.1 Run `openspec validate task-7-markdown-structure --strict` and the gate, and run `ardent init` and
      `ardent sync` by hand in a scratch copy of this repository; verify that all pass and that
      `.ardent/cache/files/README.md/structure.yaml` lists the headings of the README

## Workflow follow-up

- Archive the change into `openspec/specs/` in the same PR, and verify that the new specs `outline`, `markdown-format`,
  and `sync` exist and that `ardent-layout` holds the changed requirements.
- Finalize TASK-7 in Backlog.
