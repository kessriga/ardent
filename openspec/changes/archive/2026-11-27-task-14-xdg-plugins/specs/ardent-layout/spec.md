# Ardent layout delta

## MODIFIED Requirements

### Requirement: Plugin locations

For each enabled format, Ardent SHALL use the complete package at `.ardent/plugins/<name>/` if present. When that entry
is absent, it SHALL look under `$XDG_CONFIG_HOME/ardent/plugins/<name>/`. Both locations SHALL use the same package
format and verification rules. The release SHALL supply Markdown and Rust as separate packages. The CLI SHALL embed
neither plugin.

#### Scenario: Pinned Markdown plugin

- **WHEN** the project and user each install Markdown and the project package verifies at version `0.0.9`
- **THEN** a sync of `README.md` writes `format: markdown`, `format_version: 0.0.9` and the project component's digest
  into its stamp

#### Scenario: New project without installed plugins

- **WHEN** a new project enables Markdown and Rust and neither scope supplies either package
- **THEN** sync warns about both missing formats, scans zero files and exits with status 0

#### Scenario: Two projects share one package

- **WHEN** two projects enable Markdown, only the user installs it, and each project contains a `README.md`
- **THEN** each sync writes its own structure with the user component's digest
- **AND** neither project gains a copy of the package

#### Scenario: Remove a project pin

- **WHEN** the project Markdown package is removed and the user copy has the same version but different component bytes
- **THEN** the next sync scans the Markdown file again and records the user component's digest

#### Scenario: Broken project pin

- **WHEN** the project Markdown package is incomplete and the user has a valid copy
- **THEN** sync reports the project package error, retains the previous Markdown structures and exits with status 1

## ADDED Requirements

### Requirement: XDG homes

Ardent SHALL use absolute `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` paths. When either variable is absent, empty or
relative, Ardent SHALL use the corresponding home-directory default, `.config` or `.cache`. Reading user settings SHALL
leave the configuration directory untouched.

#### Scenario: Default configuration home

- **WHEN** the XDG configuration home is unset and the user's home contains `.config/ardent/config.yaml`
- **THEN** sync reads settings from that file

### Requirement: User configuration

Ardent SHALL read optional schema-1 settings from `$XDG_CONFIG_HOME/ardent/config.yaml`. For an enabled format, project
settings SHALL override individual user keys, including named prefixes. A project list SHALL replace the corresponding
user list, including when the project supplies an empty list. Ardent SHALL resolve inherited patterns relative to the
project root. Absent user settings SHALL allow existing projects to sync as before.

#### Scenario: Project enables only Markdown

- **WHEN** user settings configure Rust and the project enables only Markdown
- **THEN** sync applies Markdown settings and skips Rust source files

#### Scenario: Override files and retain excludes

- **WHEN** user settings define Markdown files and exclude `vendor/`, the project overrides only Markdown files with
  `["docs/**/*.md"]`, and both `docs/intro.md` and `README.md` exist
- **THEN** sync tracks `docs/intro.md`, skips `README.md` and continues excluding `vendor/`

#### Scenario: Explicitly clear the user exclude list

- **WHEN** the user excludes `docs/` and the project sets `exclude: []`
- **THEN** a matching source under `docs/` can appear in the outline browser

#### Scenario: No user settings

- **WHEN** the user config file is absent and the project supplies settings and packages
- **THEN** sync uses the project settings and packages and leaves the user config file absent

### Requirement: Compilation cache scope

Ardent SHALL accept `compilation_cache: user` or `compilation_cache: project` independently of the selected portable
package's location. The default SHALL be `user`. Both scopes SHALL store compiled code under the user's cache home. The
project scope SHALL isolate compiled code by project root. The scope SHALL affect only reuse of compiled code.

#### Scenario: Independent installation and compilation cache scopes

- **WHEN** a reader scans the same source with the same component installed in either scope, using each cache scope
- **THEN** all four combinations produce the same outline and component digest

#### Scenario: Find or clear compiled code

- **WHEN** a reader runs a scan that needs to compile a component
- **THEN** the shared cache uses `$XDG_CACHE_HOME/ardent/compiled/shared/` and the project cache uses
  `$XDG_CACHE_HOME/ardent/compiled/projects/<project-id>/`, according to the selected scope
- **AND** Ardent derives the project ID from the canonical project root

#### Scenario: Cache unavailable or inside the checkout

- **WHEN** the selected cache is unavailable or resides inside the project
- **THEN** sync can scan the portable component in memory

#### Scenario: Separate project caches

- **WHEN** two projects select project scope and both compile the same user package
- **THEN** each project populates its own compilation cache, and clearing one preserves the other's compiled code
