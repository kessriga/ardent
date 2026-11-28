# Proposal

## Why

A reader may need brighter comments against a dark background, or want headings to stand out in the outline without
giving them the same emphasis in the source preview. TASK-18 lets readers choose both colors.

## What Changes

- Install named palettes once per user in `$XDG_CONFIG_HOME/ardent/themes/`. Set `theme: catppuccin` in the user config
  for a default or in the project config to select a different installed theme.
- Apply changes to the selector or active palette while the reader browses. Resolve each source category and outline
  node kind separately.
- Keep the visible selection marker when several kinds share a color.
- Document a working example, how to apply it, and how to restore the defaults.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ardent-layout` defines theme selection, file lookup and the built-in fallback for omitted colors.
- `outline-browser` applies the resolved colors to source text and outline labels.

## Impact

The change touches configuration, browser refresh, terminal rendering and the README. Readers can switch between saved
palettes without closing the browser. Bundled palettes, a theme picker and background colors are outside this change.
