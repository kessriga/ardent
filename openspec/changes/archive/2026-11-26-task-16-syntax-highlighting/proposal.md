# Proposal

## Why

TASK-16 helps readers distinguish Rust keywords from strings and comments, and Markdown headings from prose and code. In
the outline, a reader should also be able to distinguish node kinds while following the selected row.

## What Changes

- Let a format plugin supply syntax categories for byte ranges in the open source file.
- Use those categories in the source preview and the existing node kinds in outline labels. Keep the selection marker.
- Keep source readable and the outline navigable with a scanner-only plugin.

## Capabilities

### Modified Capabilities

- `format-components` gains an optional highlighting interface alongside the existing scanner.
- `rust-format` identifies keywords, strings and comments.
- `markdown-format` identifies headings, inline code and code blocks.
- `outline-browser` applies color while preserving the text and position of the selected passage.

## Impact

The change reaches the component interface, guest packages, format host, browser loader and terminal renderer. Theme
configuration, syntax highlighting inside Markdown code fences and persistent highlight data are outside this task.
