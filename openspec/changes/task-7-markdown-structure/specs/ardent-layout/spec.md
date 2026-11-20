# Spec Delta

## MODIFIED Requirements

### Requirement: Project root

The project root SHALL be the nearest directory, starting at the working directory and going up, that contains a
`.ardent/` directory. Every command other than `ardent init` SHALL use that project root. The search SHALL start at the
working directory with symbolic links resolved. When Ardent cannot check a directory for `.ardent/` for a reason other
than a missing entry, the command SHALL fail with a message that names the path.

#### Scenario: Command in a subdirectory

- **WHEN** a person runs an Ardent command in `docs/` of a project whose root contains `.ardent/`
- **THEN** the command uses the project root, not `docs/`

#### Scenario: No project

- **WHEN** a person runs an Ardent command other than `ardent init`, and no ancestor directory contains `.ardent/`
- **THEN** the command fails with a message that the directory is not inside an Ardent project, and writes no file

#### Scenario: Directory reached through a symbolic link

- **WHEN** `/p/docs` is a symbolic link to `/data/docs`, only `/p` contains `.ardent/`, and a person runs an Ardent
  command other than `ardent init` in `/p/docs`
- **THEN** the command fails with a message that the directory is not inside an Ardent project

#### Scenario: Directory that cannot be searched

- **WHEN** the operating system denies Ardent the permission to look for `.ardent` in a directory on the way up
- **THEN** the command fails with a message that names the path, and writes no file

### Requirement: Tracked files

Ardent SHALL track a file when the `files` patterns of exactly one enabled format match it, git's ignore rules do not
ignore it, no `exclude` pattern matches it, it is outside `.ardent/`, it is outside every nested project, and its path
is valid UTF-8. A nested project is a directory below the project root that contains its own `.ardent/` directory.

#### Scenario: Ignored by git

- **WHEN** `.gitignore` contains `target/` and `target/doc/notes.md` exists
- **THEN** Ardent does not track `target/doc/notes.md`

#### Scenario: Excluded

- **WHEN** `config.yaml` contains `exclude: ["vendor/"]`
- **THEN** Ardent does not track any file under `vendor/`

#### Scenario: Two formats match

- **WHEN** two enabled formats both match `notes.md`
- **THEN** the sync reports `notes.md` and the names of both formats
- **AND** Ardent does not track `notes.md`

#### Scenario: Path is not UTF-8

- **WHEN** a file matches a format pattern and its path is not valid UTF-8
- **THEN** the sync reports the file, and Ardent does not track it

#### Scenario: Nested project

- **WHEN** the project root holds `tools/.ardent/` and `tools/README.md`
- **THEN** the sync reports `tools/` as a nested project
- **AND** Ardent does not track `tools/README.md`
