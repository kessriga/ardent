# Sync delta

## MODIFIED Requirements

### Requirement: Source files stay unchanged

A sync SHALL write only inside the project's `.ardent/` directory and the selected trusted compilation cache. It SHALL
follow no symbolic link inside `.ardent/`. When a directory that the sync writes to is a symbolic link, the sync SHALL
report it and exit with status 1. User settings and installed user packages SHALL remain read-only to sync.

#### Scenario: Sync of a project

- **WHEN** a person runs `ardent sync` in a project with Markdown and Rust files
- **THEN** every project file and directory outside `.ardent/` and the selected compilation cache keeps its content

#### Scenario: Symbolic link in the cache

- **WHEN** `.ardent/cache/files` is a symbolic link to a directory outside the project
- **THEN** `ardent sync` reports the link and exits with status 1
- **AND** every file and directory in the linked directory stays as it was

#### Scenario: Rebuild compiled code after clearing the cache

- **WHEN** a reader clears the selected compilation cache and syncs a file that needs scanning
- **THEN** the file has the same outline as a scan of the same source with the same component before the cache was
  cleared
- **AND** both scopes' settings and installed packages retain their contents, and the other compilation cache is
  untouched
