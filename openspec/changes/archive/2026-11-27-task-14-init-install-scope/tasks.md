# Tasks

## 1. Install at the chosen scope

- [x] 1.1 Reuse the CLI XDG lookup, route selected packages to the chosen installation and enable them in the project.
      Exercise missing, existing, broken and concurrently installed shared packages through init tests. Check retry
      after a copy failure and compare pre-existing user settings and packages byte-for-byte. Run `cargo fmt --check`,
      `cargo clippy --all-targets -- -D warnings` and `cargo test`.

## 2. Present the three prompts

- [x] 2.1 Connect the terminal and line-input paths to the installation choices and document the prompts in the README.
      Test No, pressing Enter through all defaults, explicitly checking Rust per-user, a project selection, and
      cancellation at each prompt. Confirm the initialized projects sync the selected formats and scripted `--plugins`
      calls still install locally. Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
      `cargo test`.

## Workflow follow-up

- Format changed Markdown, archive the change, and update TASK-14 in PR #23.
