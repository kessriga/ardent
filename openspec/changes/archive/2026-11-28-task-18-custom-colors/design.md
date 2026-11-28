# Design

## Context

Decision-4 puts project-owned settings under `.ardent/`. TASK-14 established the user configuration directory. Ardent
already knows how to find both locations. Today the terminal renderer assigns foregrounds directly to five syntax
categories and to outline node kinds.

## Decisions

- Add an optional `theme` name to the project and user `config.yaml`. Use the existing project-over-user inheritance for
  this setting. Both selectors refer to the same per-user collection, so readers can use an installed theme across
  projects and select a different one for a particular project.
- Define the theme data and RGB values in `ardent-core` and reuse its schema-versioned YAML reader. Validate the source
  keys against the five highlight categories. Accept exact outline kind names so third-party format plugins can supply
  their own kinds. Keep terminal types in `ardent-tui`.
- In `ardent-cli`, resolve the selected name under the user's Ardent configuration directory. Require a single filename
  stem and append `.yaml` inside `themes/`. Keep theme file reads in the browser so a broken palette cannot block
  `ardent sync`.
- The browser already watches the project config and checks the user config for changes. Extend refresh to cover the
  selected file in the user theme directory, including creation, removal and atomic replacement. Resolve the complete
  new palette before replacing the displayed one. If loading fails, show the cause and keep the last usable palette (the
  built-in palette on startup). A corrected file should clear the diagnostic on refresh.
- Keep the built-in palette in `ardent-tui`. Where the renderer currently chooses a foreground, use an override when
  present and the existing foreground otherwise. Leave the selection marker on the list widget.

## File example

Save this example as `$XDG_CONFIG_HOME/ardent/themes/catppuccin.yaml` to make it available to every project you open.
The path defaults to `~/.config/ardent/themes/catppuccin.yaml`. The same heading has a different foreground in the
source and outline.

```yaml
schema: 1
source:
  heading: "#E6BB70"
  comment: "#8DAA90"
outline:
  heading: "#80BBFF"
  fn: "#B694F6"
```

Add `theme: catppuccin` at the top level of the project or user `config.yaml`, alongside `schema: 1`. To switch, create
another file such as `themes/daylight.yaml` in the user configuration directory and set `theme: daylight`. Removing the
project selector reveals the user selection. Removing both selectors restores the built-in palette and keeps all saved
themes.

## Risks and trade-offs

YAML treats an unquoted leading `#` as a comment, so the guide must quote RGB values. Terminals with limited color
support may approximate the requested foreground. Older Ardent versions reject the new `theme` config key. Remove the
selector from both configs before running an older version.
