# Spec Delta

## MODIFIED Requirements

### Requirement: Initial content

`ardent init` SHALL create `.ardent/` with exactly two files: `.gitignore` and `config.yaml`. The configuration SHALL
enable Markdown and Rust with their default patterns, and no views or excludes.

#### Scenario: Files after init

- **WHEN** a person runs `ardent init` in a directory without `.ardent/`
- **THEN** `.ardent/` contains only `.gitignore` and `config.yaml`
- **AND** `.ardent/.gitignore` contains the line `/cache/`
- **AND** `.ardent/config.yaml` contains:

```yaml
schema: 1
formats:
  markdown:
    files:
      - "*.md"
      - "*.markdown"
  rust:
    files:
      - "*.rs"
views: {}
exclude: []
```

### Requirement: Plugin locations

Ardent SHALL load enabled format plugins only from `.ardent/plugins/<name>/`. The release SHALL supply Markdown and Rust
as separate packages. The CLI SHALL embed neither plugin.

#### Scenario: Pinned Markdown plugin

- **WHEN** `.ardent/plugins/markdown/plugin.yaml` and its component agree on version `0.0.9`, and a sync scans
  `README.md`
- **THEN** the stamp of the structure of `README.md` has `format: markdown` and `format_version: 0.0.9`

#### Scenario: New project without installed plugins

- **WHEN** a person runs sync after init and before installing any package
- **THEN** sync warns that the configured Markdown and Rust plugins are missing, scans zero files and exits with status
  0
