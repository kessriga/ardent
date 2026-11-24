# Proposal

## Why

Today, `ardent init` enables Markdown and Rust, but the first sync cannot scan either format until someone copies its
package by hand. TASK-13 lets a project choose its format plugins once and use them on its first sync.

## What Changes

- Offer a terminal picker for the release packages. Install the chosen packages in `.ardent/plugins/` and enable those
  same names in `config.yaml`.
- **BREAKING** Scripts must pass a comma-separated selection, such as `ardent init --plugins markdown,rust`. Without a
  terminal or a selection, init exits with instructions. An unknown name produces a list of available choices.

## Capabilities

### Modified Capabilities

- `ardent-layout` defines the contents of a newly initialized project and how users select its packages.

## Impact

The change affects the init command in `ardent-cli`, CLI integration tests, and the README setup instructions. It uses
the existing Markdown and Rust release packages. Third-party plugin sources are outside TASK-13. TASK-14 covers user
settings, user-installed packages, and compilation cache selection.
