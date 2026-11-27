# Design

## Context

Decision-4 puts project plugins under version control. Decision-8 identifies a format plugin by name and verifies its
manifest against the installed component. The existing sync stamp already detects a component rebuild. Init knows which
packages the release supplies, how to locate them beside the executable and which files each package requires.

## Goals and boundaries

Use the same release discovery and comparison for the command and browser. Existing projects must work with their
current manifests and configuration. A reader who runs the explicit update command accepts the release copy of each
eligible package, including removal of obsolete files.

## Decisions

- Share init's release catalog, path resolution and package traversal within `ardent-cli`. Match exact installed
  directory names and format plugin identity against the release catalog. Evaluate installed packages even when a reader
  has disabled them. Skip a third-party view plugin occupying a release name. The current format has no provenance field
  to distinguish a fork that declares the same format name as the bundled plugin.
- Compare relative paths, entry types and regular-file bytes. This catches rebuilt components, changed documentation and
  obsolete files. Ignore timestamps and permissions. Retain init's rejection of symlinks and special files, check
  package roots too, and keep reads and writes inside their respective roots.
- Let the CLI browser worker check on entry and after project plugin changes. Reuse its existing filesystem watcher.
  Keep package reads off the terminal input thread and avoid rereading every component on unrelated source edits. Pass
  the notice to `ardent-tui` for the status line. Keep navigation keys visible and make the full message available in
  the existing scrollable diagnostics on narrow terminals. A missing or unreadable bundle must leave a usable project
  browsable.
- In `ardent-cli`, resolve the nearest project for `update`. On a match, return before creating temporary files or a
  lock file. If a package differs, acquire the existing project sync lock and recheck. Stage the complete candidate
  under a private directory in `.ardent/cache/tmp/`. Verify its manifest and component together with the existing
  `ardent-format` loader against the staging root before publishing it.
- Publish the package as a directory under the lock. Preserve the original until the replacement is in place. Use an
  atomic directory exchange where supported. Any fallback must restore the original at its installed path before
  reporting a publish failure. Clean up only directories this update owns. Test a failure during publication as well as
  a failure during staging. Replacement is per plugin, so a successful earlier update can stand.

## Risks and rollout

An update can replace a reader's local edits to a bundled package, including a same-name fork. Explain this in the
README beside the command. Readers can review or restore project plugin changes with Git. The existing manifest, cache
stamp and configuration formats suffice for rollout.
