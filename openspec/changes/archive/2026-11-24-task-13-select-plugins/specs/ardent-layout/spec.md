# Ardent layout changes

## MODIFIED Requirements

### Requirement: Initial content

`ardent init` SHALL write `.ardent/.gitignore` with the line `/cache/` and `.ardent/config.yaml` with schema 1, empty
views and empty excludes. It SHALL copy each selected release package into `.ardent/plugins/<name>/` and enable exactly
those names under `formats` with their default patterns.

#### Scenario: Files after init

- **WHEN** a person initializes a project with only Rust selected
- **THEN** `.ardent/plugins/rust/` contains the Rust release manifest, component and accompanying README
- **AND** `.ardent/plugins/` contains only `rust/`, and `formats` contains only `rust`
- **AND** the first sync scans `src/main.rs` and skips `README.md`

#### Scenario: No formats selected

- **WHEN** a person confirms an empty selection or runs `ardent init --plugins ''`
- **THEN** init creates a project with an empty `formats` mapping
- **AND** `.ardent/` contains only `.gitignore` and `config.yaml`

#### Scenario: A selected package cannot be installed

- **WHEN** init cannot read a file from a selected release package
- **THEN** init exits with a failure, names the file, and leaves the directory ready for another init

### Requirement: Plugin locations

Ardent SHALL load enabled format plugins only from `.ardent/plugins/<name>/`. The release SHALL supply Markdown and Rust
as separate packages. The CLI SHALL embed neither plugin.

#### Scenario: Pinned Markdown plugin

- **WHEN** `.ardent/plugins/markdown/plugin.yaml` and its component agree on version `0.0.9`, and a sync scans
  `README.md`
- **THEN** the stamp of the structure of `README.md` has `format: markdown` and `format_version: 0.0.9`

#### Scenario: New project without installed plugins

- **WHEN** a new project enables Markdown and Rust in its configuration but both project packages are missing
- **THEN** sync warns about both missing formats, scans zero files and exits with status 0

## ADDED Requirements

### Requirement: Select release format plugins during init

In a terminal, `ardent init` SHALL offer the release format plugins and let the user select any subset in one prompt. An
explicit comma-separated `--plugins` value SHALL bypass the prompt. Init SHALL resolve the selection before creating the
project.

#### Scenario: Interactive selection

- **WHEN** a person runs `ardent init` in a terminal and selects Markdown and Rust
- **THEN** init installs and enables both packages

#### Scenario: Cancel the prompt

- **WHEN** a person cancels the picker before confirming a selection
- **THEN** init leaves the directory unchanged

#### Scenario: Scripted selection

- **WHEN** a script runs `ardent init --plugins markdown,rust` with standard input closed
- **THEN** init installs and enables both packages without prompting

#### Scenario: Missing flag outside a terminal

- **WHEN** a script runs `ardent init` without a terminal or a plugin selection
- **THEN** init exits with a failure, names `--plugins`, and creates no project

#### Scenario: Unknown name mixed with a valid name

- **WHEN** a person runs `ardent init --plugins markdown,html`
- **THEN** init exits with a failure, names `html`, lists `markdown` and `rust` as available, and creates no project
