# Spec Delta

## MODIFIED Requirements

### Requirement: File errors

When a sync cannot read a tracked file, cannot parse it, or cannot write its structure file, it SHALL report the file
and the error, SHALL go on with the other files, and the command SHALL exit with status 1. A file that does not parse
SHALL keep the structure file of its last scan without a change.

#### Scenario: Unreadable source file

- **WHEN** `README.md` cannot be read, and `docs/spec.md` can
- **THEN** `ardent sync` reports `README.md`, writes the structure file of `docs/spec.md`, and exits with status 1

#### Scenario: Syntax error in one file

- **WHEN** `src/lib.rs` has a syntax error, and `src/main.rs` has none
- **THEN** `ardent sync` reports `src/lib.rs` with the line and column of the error, writes the structure file of
  `src/main.rs`, and exits with status 1

#### Scenario: Syntax error after a scan

- **WHEN** a sync wrote the structure file of `src/lib.rs`, and a person then adds the half-typed line `fn helper(`
- **THEN** the next sync reports `src/lib.rs`, and its structure file keeps its content byte for byte
- **AND** after the person completes the function, the next sync writes a structure file with the node
  `src/lib.rs#fn helper`

### Requirement: Warnings

A sync SHALL print a warning to standard error for each enabled format without a plugin, each file that two formats
match, each file path that is not UTF-8, each nested project, and each line of a `.gitignore` or `.git/info/exclude`
file that Ardent cannot parse. Ardent SHALL skip such a line, as git accepts it. Warnings alone SHALL NOT change the
exit status, and SHALL NOT stop the deletion of stale structure files.

#### Scenario: Format without a plugin

- **WHEN** `config.yaml` enables the format `html`, and Ardent has no `html` plugin
- **THEN** `ardent sync` warns that the format `html` has no plugin, tracks no file for it, and exits with status 0

#### Scenario: Ignore rule that Ardent cannot parse

- **WHEN** the project is a subdirectory of a git repository, and the `.gitignore` at the repository root contains the
  line `[z-a]`
- **THEN** `ardent sync` warns about the line, deletes stale structure files, and exits with status 0
