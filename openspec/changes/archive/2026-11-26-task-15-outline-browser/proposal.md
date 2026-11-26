# Open the browser by default and clarify its outline

## Why

Readers currently need to know a subcommand to open Ardent. Inside the browser, the outline runs into the source
preview, and the first of several list items looks like a lone item.

## What Changes

- Launch the browser with `ardent` and list the available commands with `ardent --help`.
- Draw a horizontal divider between the outline and its source preview.
- Display the first item of a multi-item list as `item[1]`.

## Capabilities

### Modified Capabilities

- `outline-browser` gains a default entry point, a visible source boundary, and one-based list item labels.

## Impact

The change affects the CLI, the terminal browser, and the user guide. Existing projects can use it immediately.
