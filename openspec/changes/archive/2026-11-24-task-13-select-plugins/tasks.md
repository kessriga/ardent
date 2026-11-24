# Tasks

Finish each item with passing `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.

## 1. Install an explicit selection

- [x] 1.1 Add `init --plugins`, resolve release package paths, copy the selection atomically, and generate matching
      config entries. Update the init and sync fixtures to use this command. Test one, both, and zero packages, the
      first sync, unknown names, and rollback on an incomplete release. Keep coverage for configured packages that go
      missing.
- [x] 1.2 Replace manual package-copy steps in the README with release layout and scripted setup instructions. Run the
      example from a temporary project with a relocated executable and compare installed files to the release.

## 2. Select packages in a terminal

- [x] 2.1 Connect the terminal picker to the same install path. Document how to select packages in the README and CLI
  help. Exercise selection and cancellation in a pseudo-terminal. Test that a pipe without the flag fails with
  `--plugins` in the diagnostic and that an existing project still gets its original error before any prompt.

## Workflow follow-up

- Format changed Markdown, validate the change, archive it, and close TASK-13 in the implementation PR.
