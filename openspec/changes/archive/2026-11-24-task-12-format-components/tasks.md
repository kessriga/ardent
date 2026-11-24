# Tasks

For each item, run its stated check and finish with `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
`cargo test`.

## 1. Build and package the guests

- [x] 1.1 Share the versioned WIT and guest query engine. Move the two grammars and queries into separate WASI
      components. Preserve engine tests, rename the shadowing syntax-error helper if retained, and verify both release
      components build and the TASK-11 spike still runs.
- [x] 1.2 Generate each package manifest from its component's metadata. Document and run the packaging commands, then
      check that both packages contain matching metadata and component bytes.

## 2. Run components through the host

- [x] 2.1 Load the versioned scanner in an isolated Wasmtime store per active format. Exercise memory and execution
      limits, denied ambient access and deterministic clocks and randomness. Verify a syntax error leaves the instance
      usable and the next file after a trap runs in a fresh instance.
- [x] 2.2 Validate flat nodes before converting them for Ardent core. Test invalid parent indices, document order,
      nesting and ranges. Run the TASK-7 and TASK-8 snapshots unchanged through the two built components. Compare paths,
      ranges, fingerprints and diagnostics.
- [x] 2.3 Document the Rust 2024 parser limitation in the Rust package. Exercise the exact declaration in an inline test
      and verify the reported line and character column.

## 3. Use installed packages during sync

- [x] 3.1 Replace native plugin construction with project package discovery. Verify schema, kind, directory name and
      metadata, respect the existing config overrides, and constrain manifest paths to their package. Test absent,
      incomplete and mismatched packages alongside a healthy format. Verify errors preserve caches by suppressing
      pruning.
- [x] 3.2 Hash the installed bytes before consulting file stamps, cache successful package verification, and reuse
      trusted compiled code. Test a same-version replacement, a native stamp, a changed manifest matching no files and a
      missing verification record. Verify a current project syncs without starting Wasmtime.
- [x] 3.3 Exercise actual syncs using installed packages. Check prior structure bytes and exit status on syntax errors,
      malformed results and traps. Check recovery on the following file of the same format and continued scanning of
      other formats.
- [x] 3.4 Update the README, crate guide and technical terminology to match the installed packages. Refresh the Markdown
      and Rust spec purpose text when syncing the approved deltas. Run the documented manual installation on a new
      project and verify init still writes exactly two files.

## 4. Verify the integrated change

- [x] 4.1 Inspect `cargo tree -p ardent-cli -e normal` for grammar dependencies, confirm neither component is embedded,
      and repeat the documented sync with packages absent and present. Format changed Markdown, excluding snapshot
      inputs, and validate the OpenSpec change.

## Workflow follow-up

- Archive the change, check its merged specs, and close the Backlog task in the implementation PR.
