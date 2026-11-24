# Spec Delta

## MODIFIED Requirements

### Requirement: Plugin identity

The packaged Markdown format component SHALL have the name `markdown` and the version `0.1.0`. Its default file patterns
SHALL be `*.md` and `*.markdown`.

#### Scenario: Stamp of a Markdown file

- **WHEN** a sync scans `README.md` with the installed Markdown component
- **THEN** the stamp of its structure file has `format: markdown` and `format_version: 0.1.0`
