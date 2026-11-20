# Tasks

## 1. Atomic write

- [x] 1.1 Add `tempfile` to `ardent-core` and the module `layout` with the `.ardent`, `cache`, and `cache/tmp` paths and
      `write_atomic`; verify with unit tests that a new file gets the content, an existing file gets the new content,
      and `cache/tmp/` holds no file afterwards, and that `cargo fmt --check`,
      `cargo clippy --all-targets -- -D warnings`, and `cargo test` pass

## 2. `ardent init`

- [x] 2.1 Add `clap` to `ardent-cli`, the `init` subcommand, and the modules `project_root` and `init` that follow the
  steps in design.md; verify with unit tests that the root search finds the nearest `.ardent/` directory and skips a
  file named `.ardent`, that `ardent --help` lists `init`, and that the gate passes
- [x] 2.2 Add integration tests that run the binary in a temporary directory: a new project gets exactly
      `.ardent/.gitignore` and `.ardent/config.yaml` with the bytes from "Initial content" and nothing else changes; an
      existing `.ardent/` directory, an `.ardent` file, or a `.ardent/` directory in a parent directory gives a nonzero
      exit, a message, and an unchanged tree; verify with `cargo test` and the gate
- [ ] 2.3 Update `README.md` so the status and "How it should work" describe the working `ardent init`; verify with
  `rumdl fmt --no-cache --enable MD013 README.md`

## 3. Checks

- [ ] 3.1 Run `openspec validate task-6-ardent-init --strict` and the gate, and verify that all pass

## Workflow follow-up

- Archive the change into `openspec/specs/` in the same PR, and verify that `openspec/specs/ardent-layout/spec.md` holds
  the two new requirements.
- Finalize TASK-6 in Backlog.
