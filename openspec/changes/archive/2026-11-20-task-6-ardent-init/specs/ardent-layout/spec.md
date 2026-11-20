# Spec Delta

## ADDED Requirements

### Requirement: Init in an existing project

When the working directory already contains a file or directory named `.ardent`, `ardent init` SHALL fail with a message
that names that path, and SHALL change no file.

#### Scenario: Second init

- **WHEN** a person runs `ardent init` in a directory that already contains `.ardent/`
- **THEN** the command exits with a nonzero status and a message that `.ardent` already exists
- **AND** every file and directory in the working directory keeps its content

#### Scenario: File named .ardent

- **WHEN** a person runs `ardent init` in a directory that contains a file named `.ardent`
- **THEN** the command exits with a nonzero status and a message that `.ardent` already exists
- **AND** the file keeps its content

### Requirement: Init inside a project

When a parent directory of the working directory contains a `.ardent/` directory, `ardent init` SHALL fail with a
message that names the nearest such directory as the project root, and SHALL change no file.

#### Scenario: Init in a subdirectory

- **WHEN** a person runs `ardent init` in `docs/` of a project whose root contains `.ardent/`
- **THEN** the command exits with a nonzero status and a message that names the project root
- **AND** `docs/` contains no `.ardent`

### Requirement: Init writes only inside .ardent/

`ardent init` SHALL create, change, or delete no file or directory outside `.ardent/`.

#### Scenario: Directory with source files

- **WHEN** a person runs `ardent init` in a directory that contains `README.md` and `src/main.rs`
- **THEN** `.ardent/` is the only new entry in the working directory
- **AND** `README.md` and `src/main.rs` keep their content
