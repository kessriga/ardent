# Ardent layout delta

## RENAMED Requirements

- FROM: `### Requirement: Init writes only inside .ardent/`
- TO: `### Requirement: Init write locations`

## MODIFIED Requirements

### Requirement: Init write locations

`ardent init` SHALL limit writes to the new project's `.ardent/` directory unless the person selects per-user and at
least one package. In that case init MAY also create the directories and files needed for installation under the user's
Ardent plugin directory. It SHALL preserve user settings, existing user packages and project source files.

#### Scenario: Directory with source files

- **WHEN** a person initializes a project containing `README.md` and `src/main.rs` with per-project packages
- **THEN** `.ardent/` is the only new entry in the working directory
- **AND** both source files and the user's configuration directory keep their contents

#### Scenario: User configuration already exists

- **WHEN** a person installs Rust per-user while creating a project and already has an XDG Ardent settings file
- **THEN** the user settings file keeps its original contents and the project config enables Rust

### Requirement: Initial content

`ardent init` SHALL write `.ardent/.gitignore` with the line `/cache/` and a schema-1 `.ardent/config.yaml` with empty
views and excludes. It SHALL enable exactly the selected names under `formats` with their default patterns. Per-project
installation SHALL copy complete release packages into `.ardent/plugins/<name>/`. Per-user installation SHALL make each
selected package available under the user's XDG `ardent/plugins/<name>/` directory.

#### Scenario: Files after init

- **WHEN** a person chooses per-project and selects only Rust
- **THEN** `.ardent/plugins/rust/` contains the release manifest, component and accompanying README
- **AND** the project enables only Rust and its first sync scans `src/main.rs` and skips `README.md`

#### Scenario: Shared installation in two projects

- **WHEN** a person selects Rust per-user in two new projects and Rust is initially absent from the user installation
- **THEN** the first init copies the complete Rust release package and the second reuses it
- **AND** both projects enable Rust, create no project package, and can sync a Rust source file

#### Scenario: Existing shared package

- **WHEN** the user already has a valid Rust format package with a different version or additional files
- **THEN** selecting Rust per-user enables the existing package and preserves all its files

#### Scenario: No formats selected

- **WHEN** a person declines additional plugins, confirms an empty checklist or runs `ardent init --plugins ''`
- **THEN** init creates a project with an empty `formats` mapping and only `.gitignore` and `config.yaml` in `.ardent/`
- **AND** init creates no user package directory

#### Scenario: A selected package cannot be installed

- **WHEN** init cannot install or reuse a selected package
- **THEN** init reports the affected path or package, exits unsuccessfully and leaves the project ready for another init
- **AND** existing shared packages stay intact and no partially copied package becomes visible

### Requirement: Select release format plugins during init

In a terminal, `ardent init` SHALL first ask `Install format plugins? [Y/n]`. Yes SHALL lead to an installation scope
picker followed by a plugin checklist. No SHALL skip both. Init SHALL resolve the selection before writing the project
or shared packages. An explicit `--plugins` value SHALL use project scope and bypass all prompts.

#### Scenario: Interactive selection

- **WHEN** the person presses Enter at the first question
- **THEN** the picker asks `Where should Ardent install them?` and highlights `For my user account` above
  `In this project`
- **AND** confirming the scope opens `Select format plugins` with every plugin unchecked
- **AND** Space toggles the focused plugin and Enter confirms the checked items. Pressing Enter immediately selects none

#### Scenario: Cancel the prompt

- **WHEN** the person cancels at any of the three prompts
- **THEN** init restores the terminal and leaves the project and user plugin directories unchanged

#### Scenario: Scripted selection

- **WHEN** a script runs `ardent init --plugins markdown,rust` with standard input closed
- **THEN** the command installs and enables both packages in the project without prompting

#### Scenario: Missing flag outside a terminal

- **WHEN** a script runs `ardent init` without a terminal or a plugin selection
- **THEN** init exits with a failure, names `--plugins` and creates no project

#### Scenario: Unknown name mixed with a valid name

- **WHEN** a person runs `ardent init --plugins markdown,html`
- **THEN** init exits with a failure, names `html`, lists `markdown` and `rust` as available and creates no project
