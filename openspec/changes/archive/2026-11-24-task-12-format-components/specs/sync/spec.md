# Spec Delta

## MODIFIED Requirements

### Requirement: Sync command

`ardent sync` SHALL run a sync in the project of the working directory. It SHALL write a structure file for each tracked
file that has a format plugin, and SHALL print one summary line with the number of tracked files and the number of files
it scanned.

#### Scenario: First sync

- **WHEN** a person runs `ardent sync` in a new project with the Markdown component installed that holds `README.md` and
  `docs/spec.md`
- **THEN** `.ardent/cache/files/README.md/structure.yaml` and `.ardent/cache/files/docs/spec.md/structure.yaml` exist
- **AND** the command prints that it tracks 2 files and scanned 2, and exits with status 0

### Requirement: Changed format plugin

When the format name, the format version, or the format hash in a stamp differs from the plugin that tracks the file
now, a sync SHALL scan the file again.

#### Scenario: Older plugin version

- **WHEN** the structure file of `README.md` has `format_version: 0.0.9`, and the installed Markdown component has
  version `0.1.0`
- **THEN** the next sync scans `README.md` and writes `format_version: 0.1.0`

#### Scenario: Structure from a native plugin

- **WHEN** a file has a structure stamped with the old native plugin hash and its component is now installed
- **THEN** sync scans the file again and writes the digest of the installed component even when the source metadata and
  plugin version match

### Requirement: Source files stay unchanged

Within the project a sync SHALL write only inside `.ardent/`. The host MAY also write compiled code to Wasmtime’s
trusted local compilation cache. It SHALL follow no symbolic link inside `.ardent/`: when a directory that the sync
writes to is a symbolic link, the sync SHALL report it and exit with status 1.

#### Scenario: Sync of a project

- **WHEN** a person runs `ardent sync` in a project with Markdown and Rust files
- **THEN** every project file and directory outside `.ardent/` keeps its content

#### Scenario: Symbolic link in the cache

- **WHEN** `.ardent/cache/files` is a symbolic link to a directory outside the project
- **THEN** `ardent sync` reports the link and exits with status 1
- **AND** every file and directory in the linked directory stays as it was

## ADDED Requirements

### Requirement: Verify packages before file discovery

Sync SHALL verify each enabled, installed package before applying project overrides or using its defaults. It SHALL
check changed manifest or component bytes even when no file matches. It SHALL reuse successful verification for
identical manifest bytes, component bytes and supported interface. A sync that needs neither verification nor a scan
SHALL finish without starting a component runtime.

#### Scenario: Metadata mismatch while other formats are usable

- **WHEN** the installed Rust manifest changes its default pattern to `*.txt`, the component still declares `*.rs`, the
  project has no text files, and a Markdown file changes
- **THEN** sync reports the Rust plugin, retains the existing Rust structures and stamps, updates the Markdown file's
  structure, skips stale-structure deletion and exits with status 1

#### Scenario: All files and packages are current

- **WHEN** a person repeats a successful sync with the same packages, verification records and unchanged files
- **THEN** sync reports zero scanned files, preserves their structures and makes no component calls
