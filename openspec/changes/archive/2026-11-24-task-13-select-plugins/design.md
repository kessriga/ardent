# Design

## Context

Decision-8 defines Markdown and Rust as separate release packages. The checkout already contains each manifest and
component under `plugins/<name>/`. The Rust package also includes its parser notes. Init currently writes a fixed
configuration for both formats. The sync tests copy the components into each test project themselves.

## Goals and scope

Keep terminal input, flag handling, and package installation in `ardent-cli`. The schema already supports a mapping from
a format name to empty settings, which tells sync to use the package defaults. The existing package format and the sync
loader need no changes.

## Decisions

- Check for an existing or enclosing Ardent project before prompting. Resolve either input path to the same set of
  release names. Use the set both to install packages and to generate `formats`. Treat a deliberately empty selection as
  a project with no formats. An interrupted prompt creates nothing.
- For distributed builds, look for `plugins/` beside the executable. Document how to place the binary and its release
  packages together. For Cargo development builds, allow the checkout's package directory as a fallback. Never search
  the project being initialized for release assets. A relocated release must work without its build checkout.
- Install all files shipped in each selected package, including the Rust README. Use the existing atomic writer and
  cleanup path. Once init creates `.ardent/`, a failed install follows the same rollback as a failed config write.
- Write empty settings for each selected format so the first sync gets its patterns from the installed package. Existing
  projects with explicit patterns continue to use those overrides.

## Risks and migration

Scripts that currently run bare `ardent init` must specify `--plugins`. Cover this in CLI help and the README. Test
release lookup from an unrelated working directory and check that a failed package copy leaves init safe to retry.
