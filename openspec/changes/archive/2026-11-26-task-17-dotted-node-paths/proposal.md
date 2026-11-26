# Dotted node paths

## Why

TASK-17 gives readers a consistent way to identify repeated nodes across formats.

## What Changes

- Number generic labels and repeated sibling names with one-based dotted suffixes.
- Display a singleton without its position. Escape literal dots in node paths.
- Rebuild derived structures with the new paths and display labels.

## Capabilities

### Modified Capabilities

- `outline` defines the shared path rule.
- `outline-browser` displays sibling positions for every format.
- `markdown-format`, `rust-format` and `ardent-schemas` use the new addresses.

## Impact

**Breaking** for references to changed node paths. Migration of saved addresses is outside this task.
