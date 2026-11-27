# Design

## Context

Decision-8 specifies project precedence for settings and packages and allows both compilation cache scopes. Today
`sync::run_at` reads a single project config, validates patterns, takes the project lock, recovers interrupted updates,
then discovers packages. Browse receives the format plugins from this same sync. Update also calls package discovery but
operates only on project installations.

## Goals and scope

Let projects share portable packages and settings. Keep structure files, verification records, stamps and update
recovery local to each project. This change covers format plugins. Users can install one by copying its release
directory into their own configuration directory.

## Proposed design

### Configuration

Resolve `ardent/config.yaml` and `ardent/plugins/` relative to the user's XDG configuration home in `ardent-cli`. Honor
absolute XDG paths. For absent, empty or relative paths, use the home-directory defaults `.config` and `.cache`. If no
home can be resolved, project-only operation still works.
[XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/)

Keep parsing and merging in `ardent-core`. Require schema 1 in each supplied file, validate each layer and name its path
in any diagnostic. A missing user file is normal. Preserve the difference between an omitted sequence and an explicit
empty sequence until after merging. A project sequence replaces the user sequence as a unit. A project prefix overrides
the same prefix name. Apply the selected component's defaults after resolving explicit settings. Project patterns,
including inherited ones, are relative to the project root.

### Packages and updates

Keep two package roots in the CLI. Use `ardent-format`'s existing loader and verification for the selected package. Look
up the project entry first and consult the user root only if the entry is absent. Keep recovery ahead of lookup so a
pending or failed update cannot silently switch the project to a different component. Sync, browse and highlighting must
use the same selection. Keep release update checks and writes limited to packages installed in the project.

### Compilation cache

Propose an optional schema-1 setting, `compilation_cache: user` (default) or `compilation_cache: project`, with the same
project-over-user precedence as other settings. Pass the selected directory into the lazy Wasmtime runtime in
`ardent-format`. Keep both caches in user-controlled storage because Wasmtime caches native code. Use
`$XDG_CACHE_HOME/ardent/compiled/shared/` for the shared cache and
`$XDG_CACHE_HOME/ardent/compiled/projects/<project-id>/` for a project, where the ID is a digest of the canonical
project root. A moved checkout gets a cold cache. Neither scope reads compiled code supplied by a checkout. If the
selected cache is unavailable, retain the current ability to compile in memory.

## Format selection

The project lists each enabled format. The user file supplies defaults for those names. An empty project selection
therefore continues to track no files, even when the user has installed and configured a format.
