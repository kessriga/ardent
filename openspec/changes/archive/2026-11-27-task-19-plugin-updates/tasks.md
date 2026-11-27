# Tasks

After each task, run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.

## 1. Compare project and release packages

- [x] 1.1 Share release package discovery and traversal with init. Test identical packages, a component rebuilt under
  the same version, changed or removed nested files, and rejected symlinks. Run the existing init package tests.

## 2. Update the installed copies

- [x] 2.1 Add `ardent update` to the CLI. Stage and validate each replacement, then publish it under the project lock.
      Test from a subdirectory with a disabled package, an absent package, a locally edited same-name fork and a
      separately named third-party plugin. Verify that update replaces the fork without prompting. Compare the installed
      trees and configuration before and after, including file modification times across a second run. Document the
      command and the treatment of local edits in the README.
- [x] 2.2 Exercise invalid release contents and a publication failure. Check that stderr names the affected plugin, the
      previous package still scans a fixture and a concurrent sync reads a complete package.

## 3. Display and clear the notice

- [x] 3.1 Connect the comparison to the browser worker and status display. Test the names and command on entry,
      navigation on a narrow terminal, and the notice clearing after an external update. Verify both default and
      explicit browser entry points and document the notice in the README.

## Workflow follow-up

- Format the changed Markdown and validate the OpenSpec change. Archive it, verify the resulting specs and close TASK-19
  in the implementation PR.
