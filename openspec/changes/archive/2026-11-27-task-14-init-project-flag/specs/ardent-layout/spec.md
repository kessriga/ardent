# Ardent layout delta

## ADDED Requirements

### Requirement: Explicit project scope for scripted init

`ardent init` SHALL accept `--project` with `--plugins NAMES` to install the selected format plugins in the project. The
command SHALL document the flag in its help. It SHALL require `--plugins` when `--project` is present.

#### Scenario: Select a project installation in a script

- **WHEN** a script runs `ardent init --plugins rust --project` with standard input closed
- **THEN** init installs the bundled Rust package in `.ardent/plugins/rust/` and enables only Rust in the project
- **AND** the user plugin directory keeps its prior contents, including when a shared Rust package already exists

#### Scenario: Project flag without a plugin list

- **WHEN** a person runs `ardent init --project`
- **THEN** argument parsing exits with a usage error that names `--plugins` and creates no project or user packages
