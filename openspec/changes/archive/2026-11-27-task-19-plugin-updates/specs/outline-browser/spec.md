# Spec Delta

## ADDED Requirements

### Requirement: Show available release plugin updates

On opening a project, the browser SHALL name installed release format plugins whose packages differ from the running
release and show the command `ardent update`. An available update SHALL allow normal browsing and successful exit. The
browser SHALL clear the notice when a refresh finds that all installed release packages match.

#### Scenario: A reader has an older build with the same plugin version

- **WHEN** a reader runs `ardent` with an installed Markdown component that differs from the bundled component
- **AND** the manifest version is identical and Rust is current
- **THEN** the browser names Markdown and shows `ardent update` without listing Rust as outdated

#### Scenario: A reader updates the project from another terminal

- **WHEN** a reader runs `ardent update` while the browser displays the notice
- **THEN** the next refresh clears the notice and the reader can continue browsing

#### Scenario: A reader opens a project with no eligible updates

- **WHEN** the reader runs `ardent browse` and each bundled plugin is either absent from the project or current
- **THEN** the browser shows no update notice
