# Design

## Context

Decision-8 defines project and per-user format packages. The CLI can already read both. Its init picker currently
returns a list of release packages, and init copies every item into the new project. The release copier accepts a
destination directory, but writes individual files there. A shared installation can already serve other projects.

## Goals and scope

Complete the three interactive choices before publishing any files. Use the same XDG configuration home as sync. Keep
existing scripts working. The scope choice applies to the entire selection and does not set the compilation cache.

## Decisions

In `ardent-cli`, return the selected installation scope alongside the selected names. Keep the existing keyboard input,
signal restoration and narrow-terminal support for all three prompts. On Windows pipe terminals, ask the same three
questions using line input and show the defaults. Explicit `--plugins` calls keep their project destination.

Keep the project root checks before and after prompting. Write the selected names to the project's config at both
scopes. Move the CLI's XDG configuration-directory lookup to a location shared by init and sync. Resolve the user
destination only when at least one per-user package was selected.

For a missing per-user package, copy the complete release package to private staging on the destination filesystem.
Publish it without replacing an existing entry. If a concurrent init installs it first, reuse the completed package.
Verify an existing package before reporting success. A broken package produces a diagnostic with its path and keeps its
contents. Keep verification records and temporary files out of the installed package.

## Failure and retry

A failed init removes the project it started. Already published, complete user packages remain available to other
projects. Report any newly installed packages that were retained, so a failure cannot claim that nothing changed. A
retry can reuse them. Cancellation before confirmation has no filesystem effects.
