# Tasks

Each task ends with a passing `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.

## 1. Decisions

- [x] 1.1 Record decision-6 (`@annotation` captures) and decision-7 (syntax errors keep the last structure file) with
      `backlog decision create`; verify that `backlog decision list` shows both as accepted

## 2. Outline engine

- [ ] 2.1 Read `@annotation` captures in `engine.rs` and widen the range of the candidate to cover them; verify with
      unit tests for the scenarios "Doc comment before a function" and "Match without annotations" of the `outline`
      delta
- [ ] 2.2 Return `ScanError::Syntax { line, column }` from `FormatPlugin::scan` for a tree with an `ERROR` or `MISSING`
      node, with the column counted in characters; verify with unit tests for the scenario "Half-typed function", an
      error on the first line, and an error after a non-ASCII character
- [ ] 2.3 Move the format hash rule of built-in plugins from `markdown.rs` into `engine.rs`; verify that the Markdown
      tests pass and that the Markdown format hash does not change

## 3. Rust plugin

- [ ] 3.1 Add `tree-sitter-rust = "=0.24.2"` and the module `rust` with `rust/outline.scm` for items, labels, and
      annotations; verify with unit tests for identity, the pin, the spec example of `ardent-schemas`, and the scenarios
      of "Item nodes", "Item labels", "Impl block labels", "Extern block labels", "Nesting of items", "Doc comments and
      attributes", and "Rust code that is not a node" of the spec `rust-format`
- [ ] 3.2 Add the patterns for fields, tuple fields, and enum variants; verify with unit tests for the scenarios of
      "Fields and variants" and "Attribute on a tuple field"
- [ ] 3.3 Add `insta` snapshot tests in `tests/rust.rs` for files in `tests/rust/` with inline modules, impl blocks,
      trait impls, fields, variants, and doc comments, and for the error message of a file with a syntax error; verify
      with `cargo insta review` that each snapshot shows the expected outline

## 4. Sync

- [ ] 4.1 Load the Rust plugin in `builtin_plugins` of `ardent-cli`; verify with an integration test that a sync of a
      new project writes the structure file of `src/main.rs` with `format: rust`
- [ ] 4.2 Change the test "Format without a plugin" to enable `html`, and add integration tests for the scenarios
      "Syntax error in one file" and "Syntax error after a scan" of the `sync` delta; verify that they pass
- [ ] 4.3 Update `README.md`: the status line and the note that `ardent sync` warns about the `rust` format; verify with
      `rumdl fmt --no-cache --enable MD013 README.md`

## 5. Verification

- [ ] 5.1 Run `ardent sync` on a copy of this repository; verify that it scans every `.rs` and `.md` file, reports no
      syntax error, and that a second run scans 0 files
- [ ] 5.2 Run `openspec validate task-8-rust-structure --strict`; verify that it passes

## Workflow follow-up

- Archive the change with `/opsx:archive` in the same PR as the code, and run `openspec validate --specs --strict`.
- Finalize TASK-8 in Backlog in the same PR.
