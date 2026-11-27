# Proposal

## Why

TASK-14 lets someone who uses Ardent in several projects install a format plugin once and share its settings. Decision-8
gives a project precedence when it supplies its own package or overrides a setting.

## What Changes

- Read optional user settings and format packages under the user's XDG configuration directory.
- Resolve settings by key. Select one complete package for each enabled format and stamp structures with the digest of
  the component that produced them.
- Let a project use its own compilation cache or share one across the user's projects, with either installation scope.
  Clearing compiled code must preserve both scopes' settings and packages.
- Keep installation and update commands scoped to project packages. Document how to install a user package by copying
  its complete release directory. A user package manager and view plugin execution are outside TASK-14.

## Capabilities

### Modified Capabilities

- `ardent-layout` defines settings precedence, format package lookup and the two compilation cache scopes.
- `sync` defines how package selection affects verification, existing structures and permitted writes.

## Impact

The work affects configuration in `ardent-core`, package discovery and Wasmtime setup in `ardent-format`, and the
configuration and package inputs that `ardent-cli` supplies to sync and browse. Existing project packages and structure
stamps can keep their current format.
