# Proposal

## Why

TASK-20 lets readers compare properties of several nodes while keeping the outline and source preview in view. A Rust
reader can also select the part of a signature that declares a type.

## What Changes

- In the outline, Tab opens or closes the selected node’s inline properties. Each node keeps its own state.
- **BREAKING** Shift+Tab switches between the explorer and the open document. The key hints and reader guide explain
  both bindings.
- Each format plugin opts individual properties into inline display. Markdown checkbox items expose `checked`. Rust
  declarations expose written visibility, types or bounds where applicable.
- Rust functions gain selectable children for their generic parameters, inputs and explicit return clause.

A modal for inspecting all properties, stored properties from view plugins, and property editing belong to later work.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `outline-browser` handles the property rows, navigation and key bindings.
- `format-components` lets a format plugin supply properties with an explicit inline choice while keeping scanner-only
  packages usable.
- `markdown-format` exposes the state of a list item’s own checkbox.
- `rust-format` describes declaration properties, signature children and their source ranges.

## Impact

The change reaches the format plugin interface, the Markdown and Rust guests and release packages, the format host, CLI
document loading, and terminal rendering and input. Signature children appear wherever Ardent reads the Rust outline.
Inline rows are presentation only.
