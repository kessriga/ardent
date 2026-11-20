# Tasks

## 1. Decision and specs

- [x] 1.1 Check that decision-4 and the specs `ardent-layout` and `ardent-schemas` cover each acceptance criterion of
  TASK-5; verify by mapping each criterion to a section or requirement in the TASK-5 notes
- [x] 1.2 Check the worked examples; verify that every YAML example parses, that the example sources match the generator
  input byte for byte, and that sizes and hashes of the view files match the backlink index example
- [x] 1.3 Run `openspec validate task-5-ardent-layout --strict` and verify that it reports the change as valid

## 2. Project documents

- [x] 2.1 Update `README.md`: mark the layout question as decided in decision-4, describe `views/` and `cache/` under
  "How it should work", and add *view entry* to the glossary; verify with `rumdl fmt --no-cache --enable MD013`
- [x] 2.2 Add *view entry* to the term list in `AGENTS.md` and `openspec/config.yaml`; verify with `rumdl` on
      `AGENTS.md`

## 3. Gate

- [x] 3.1 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`, and verify that all
  three pass

## Workflow follow-up

- Archive the change into `openspec/specs/` in the same PR, and verify that `openspec/specs/ardent-layout/spec.md` and
  `openspec/specs/ardent-schemas/spec.md` exist.
- Finalize TASK-5 in Backlog.
