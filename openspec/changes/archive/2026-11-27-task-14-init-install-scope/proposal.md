# Proposal

## Why

TASK-14 lets projects read a shared format plugin. Today the init picker copies every selected package into the project.
Someone who wants to share Rust across two projects must copy it by hand and edit each project's configuration.

## What Changes

- Ask `Install format plugins? [Y/n]`. No creates an empty project. Yes opens the installation scope picker.
- Offer per-user first, then per-project. Show the bundled plugin checklist with every plugin unchecked.
- Enable the selected names in the new project. Install missing shared packages once and reuse existing user packages.
- Keep scripted `ardent init --plugins …` project-local. Shared package updates and cache selection are outside this
  change.

## Capabilities

### Modified Capabilities

- `ardent-layout` defines the init prompts, the files created at each scope and the write boundary for per-user
  installation.

## Impact

The change affects the CLI picker, release package installation, the README and init tests. It uses the XDG package
location and lookup precedence already specified by decision-8 and TASK-14.
