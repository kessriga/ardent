# Tasks

## 1. Select and load a named theme

- [x] 1.1 Add the optional config selector and load the named file from the user theme directory. Test selector
      precedence, a project-local file with the same name, invalid stems, all five source categories, plugin-defined
      outline kinds, built-in fallback and missing or invalid files. Document global installation, the XDG default and a
      working `catppuccin.yaml` with its selector. Pass `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
      and `cargo test`.

## 2. Apply colors while browsing

- [x] 2.1 Render the README example and verify that one heading has distinct source and outline colors and the marker
  moves between identically colored rows. Pass `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo test`.
- [x] 2.2 Refresh when project or user selection changes or an active palette is created, edited, replaced or removed.
      Exercise the browser refresh path with two saved themes. Verify position, error recovery and removal of both
      selectors to restore defaults. Document switching and restoring while browsing. Format changed Markdown and pass
      `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.

## Workflow follow-up

- Archive the change with its spec updates and close TASK-18 in the feature PR.
