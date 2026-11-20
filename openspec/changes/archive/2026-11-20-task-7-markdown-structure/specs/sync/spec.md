# Spec Delta

## Purpose

Defines the `ardent sync` command: how a sync finds the tracked files whose structure is stale, re-scans them, writes
their structure files, deletes the structure files of files that are no longer tracked, and reports problems. The rules
follow decision-2. M1 has no views, so this sync has no links to re-resolve.

## ADDED Requirements

### Requirement: Sync command

`ardent sync` SHALL run a sync in the project of the working directory. It SHALL write a structure file for each tracked
file that has a format plugin, and SHALL print one summary line with the number of tracked files and the number of files
it scanned.

#### Scenario: First sync

- **WHEN** a person runs `ardent sync` in a new project that holds `README.md` and `docs/spec.md`
- **THEN** `.ardent/cache/files/README.md/structure.yaml` and `.ardent/cache/files/docs/spec.md/structure.yaml` exist
- **AND** the command prints that it tracks 2 files and scanned 2, and exits with status 0

### Requirement: Unchanged files

A sync SHALL NOT scan a tracked file again when its structure file has the current schema, the stamp has the format
name, version, and format hash of the current plugin and the size and modification time of the file, and the
modification time of the file is earlier than the modification time of its structure file.

#### Scenario: Second sync

- **WHEN** a person runs `ardent sync` twice and changes no file between the runs
- **THEN** the second run reports 0 scanned files
- **AND** every structure file keeps its content byte for byte

### Requirement: Changed files

When the size or the modification time of a tracked file differs from its stamp, a sync SHALL hash the file. When the
hash equals the stamp, the sync SHALL write the new size and time into the stamp and keep the nodes. When the hash
differs, the sync SHALL scan the file again and replace its structure file.

#### Scenario: Touched file

- **WHEN** a person changes the modification time of `README.md` and not its content, and runs `ardent sync`
- **THEN** the stamp of `README.md` has the new `mtime` and the old `hash`, and its nodes are unchanged

#### Scenario: Edited file

- **WHEN** a person adds a heading to `README.md` and runs `ardent sync`
- **THEN** the structure file of `README.md` lists the new heading and has the new `hash`

### Requirement: Edits within one clock tick

When the modification time of a tracked file is not earlier than the modification time of its structure file, a sync
SHALL hash the file as if its stamp differed. A coarse clock can give an edit that keeps the size of the file the
modification time of the content that the last sync read.

#### Scenario: Edit within one clock tick

- **WHEN** an edit keeps the size of `README.md`, and a coarse clock gives `README.md` and its structure file the
  modification time of the old content
- **THEN** the next sync scans `README.md` again

### Requirement: Changed format plugin

When the format name, the format version, or the format hash in a stamp differs from the plugin that tracks the file
now, a sync SHALL scan the file again.

#### Scenario: Older plugin version

- **WHEN** the structure file of `README.md` has `format_version: 0.0.9`, and the built-in Markdown plugin has version
  `0.1.0`
- **THEN** the next sync scans `README.md` and writes `format_version: 0.1.0`

### Requirement: Invalid structure file

When a structure file is not valid YAML, has an unknown schema, or has a key that the schema does not define, a sync
SHALL scan the source file again and replace the structure file.

#### Scenario: Broken structure file

- **WHEN** a person replaces `.ardent/cache/files/README.md/structure.yaml` with the text `nodes: [`
- **THEN** the next sync writes a valid structure file for `README.md`

### Requirement: Stale structure files

A sync SHALL delete each structure file whose source file is not tracked, and SHALL remove each directory under
`.ardent/cache/files/` that becomes empty. When the sync cannot read a directory of the project, it SHALL report the
directory, SHALL delete no structure file, and the command SHALL exit with status 1.

#### Scenario: Deleted source file

- **WHEN** a person deletes `docs/spec.md` and runs `ardent sync`
- **THEN** `.ardent/cache/files/docs/` no longer exists

#### Scenario: Unreadable directory

- **WHEN** the sync cannot read the directory `docs/`, which holds the tracked file `docs/spec.md` from the last sync
- **THEN** the sync reports `docs/`
- **AND** `.ardent/cache/files/docs/spec.md/structure.yaml` still exists

### Requirement: Deletion before scans

A sync SHALL delete stale structure files before it writes structure files. A file renamed to another letter case then
gets a structure file under its new name, also on a file system that ignores letter case.

#### Scenario: New letter case

- **WHEN** a person renames `README.md` to `Readme.md` and runs `ardent sync`
- **THEN** `.ardent/cache/files/` holds a directory with the name `Readme.md`, and no directory with the name
  `README.md`
- **AND** the structure file in it has the node path `Readme.md`

### Requirement: File errors

When a sync cannot read a tracked file or write its structure file, it SHALL report the file and the error, SHALL go on
with the other files, and the command SHALL exit with status 1.

#### Scenario: Unreadable source file

- **WHEN** `README.md` cannot be read, and `docs/spec.md` can
- **THEN** `ardent sync` reports `README.md`, writes the structure file of `docs/spec.md`, and exits with status 1

### Requirement: Warnings

A sync SHALL print a warning to standard error for each enabled format without a plugin, each file that two formats
match, each file path that is not UTF-8, each nested project, and each line of a `.gitignore` or `.git/info/exclude`
file that Ardent cannot parse. Ardent SHALL skip such a line, as git accepts it. Warnings alone SHALL NOT change the
exit status, and SHALL NOT stop the deletion of stale structure files.

#### Scenario: Format without a plugin

- **WHEN** `config.yaml` enables the format `rust`, and Ardent has no `rust` plugin
- **THEN** `ardent sync` warns that the format `rust` has no plugin, tracks no `.rs` file, and exits with status 0

#### Scenario: Ignore rule that Ardent cannot parse

- **WHEN** the project is a subdirectory of a git repository, and the `.gitignore` at the repository root contains the
  line `[z-a]`
- **THEN** `ardent sync` warns about the line, deletes stale structure files, and exits with status 0

### Requirement: Invalid configuration

When `config.yaml` is not valid YAML, has no `schema` or an unknown one, has a key that the schema does not define, has
a pattern that is not valid gitignore syntax, or enables a plugin with an invalid or reserved name, `ardent sync` SHALL
fail with a message that names the file, and SHALL write no file.

#### Scenario: Typo in the configuration

- **WHEN** `config.yaml` has the key `exlude` in place of `exclude`
- **THEN** `ardent sync` exits with a nonzero status and a message that names `config.yaml` and the key `exlude`
- **AND** `.ardent/cache/` does not exist after the run, when it did not exist before

#### Scenario: Invalid pattern

- **WHEN** `config.yaml` has `exclude: ["[z-a]"]`
- **THEN** `ardent sync` exits with a nonzero status and a message that names `config.yaml`
- **AND** `.ardent/cache/` does not exist after the run, when it did not exist before

### Requirement: Source files stay unchanged

A sync SHALL create, change, or delete no file or directory outside `.ardent/`. It SHALL follow no symbolic link inside
`.ardent/`: when a directory that the sync writes to is a symbolic link, the sync SHALL report it and exit with status
1.

#### Scenario: Sync of a project

- **WHEN** a person runs `ardent sync` in a project with Markdown and Rust files
- **THEN** every file and directory outside `.ardent/` keeps its content

#### Scenario: Symbolic link in the cache

- **WHEN** `.ardent/cache/files` is a symbolic link to a directory outside the project
- **THEN** `ardent sync` reports the link and exits with status 1
- **AND** every file and directory in the linked directory stays as it was
