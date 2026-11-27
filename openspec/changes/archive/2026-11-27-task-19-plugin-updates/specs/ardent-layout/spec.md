# Spec Delta

## ADDED Requirements

### Requirement: Update installed release format plugins

From any directory in a project, `ardent update` SHALL replace each differing, installed release format plugin with a
complete copy of its package from the running release. It SHALL compare package paths and file contents, preserve
project configuration and plugins with other identities, and skip release plugins the project has not installed. A
locally modified format plugin with the same directory and manifest name as a bundled format plugin SHALL count as that
bundled plugin.

#### Scenario: A component changes without a version change

- **WHEN** the bundled Markdown component differs from the installed component and both manifests give the same version
- **AND** the reader runs `ardent update` from a project subdirectory
- **THEN** the command names Markdown as updated and the installed package matches the complete bundled package

#### Scenario: The release changes only a supporting file

- **WHEN** the release changes a nested documentation file and removes another file from its Rust package
- **THEN** running `ardent update` installs the new file contents and removes the obsolete file

#### Scenario: Project installation choices survive an update

- **WHEN** Markdown is installed but disabled, Rust is absent, and a third-party plugin is installed
- **THEN** the command updates Markdown if it differs, leaves Rust absent and preserves every third-party file
- **AND** the project configuration, including comments and overrides, stays byte-for-byte identical

#### Scenario: A reader keeps a local fork under the bundled name

- **WHEN** the project contains a locally built `markdown` format plugin with an extra file and a separate `my-markdown`
  plugin
- **AND** the reader runs `ardent update`
- **THEN** the command replaces the complete `markdown` package without prompting, including removing the extra file
- **AND** every file in `my-markdown` retains its contents

#### Scenario: All installed release packages match

- **WHEN** a reader runs `ardent update` again after a successful update
- **THEN** it reports that the installed release plugins are current, exits successfully and changes no files

### Requirement: Keep the previous package usable on update failure

If Ardent cannot read, validate or install a replacement, it SHALL report the affected plugin, return failure and leave
that plugin's previous package usable. A concurrent Ardent sync SHALL see a complete package.

#### Scenario: The bundled component cannot be loaded

- **WHEN** the installed Rust plugin works and the bundled replacement contains an invalid component
- **THEN** `ardent update` fails, names Rust, and the next sync can still use the original Rust package

#### Scenario: Publishing the staged replacement fails

- **WHEN** the candidate package is complete but Ardent cannot put it in the installed location
- **THEN** the command identifies the plugin and its original files remain at the installed location
