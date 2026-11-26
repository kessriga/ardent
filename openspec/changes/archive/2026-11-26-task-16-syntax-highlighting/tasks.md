# Tasks

Finish each numbered item by running `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.

## 1. Format plugins

- [x] 1.1 Define the optional highlighter and document its byte ranges in the guest guide. Test host validation,
      unsupported exports and trap recovery with the scanner-only fixture and malformed results.
- [x] 1.2 Implement the Rust and Markdown categories. Test keyword text inside strings and comments, heading boundaries,
      inline code and both kinds of code block. Run the native guest tests, rebuild the release and example packages,
      and exercise the packaged components through the host.

## 2. Browser

- [x] 2.1 Load highlights for the verified source and installed component in the browser. Test loading and refreshing a
      document, a changed component digest and the scanner-only fallback.
- [x] 2.2 Color source and outline labels and describe the behavior in the README. Compare terminal cell symbols and
      styles for a selected child, a token across a wrap and page boundary, wide and combining characters, tabs, CRLF
      and invalid UTF-8. Verify selection visibility on different node kinds.

## 3. Integration

- [x] 3.1 Open the packaged examples in the browser, check the Rust and Markdown categories, and repeat navigation with
      the scanner-only fixture. Format the changed Markdown and validate the OpenSpec change in strict mode.

## Workflow follow-up

- Archive the change, verify the merged specs and close TASK-16 in the implementation PR.
