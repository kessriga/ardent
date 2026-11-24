# Spec Delta

## MODIFIED Requirements

### Requirement: Plugin identity

The packaged Rust format component SHALL have the name `rust` and the version `0.1.0`. Its default file pattern SHALL be
`*.rs`.

#### Scenario: Stamp of a Rust file

- **WHEN** a sync scans `src/engine.rs` with the installed Rust component
- **THEN** the stamp of its structure file has `format: rust` and `format_version: 0.1.0`

## ADDED Requirements

### Requirement: Documented parser limit

The Rust plugin documentation SHALL identify the Rust 2024 `safe` item syntax in `unsafe extern` blocks that
tree-sitter-rust 0.24.2 rejects. It SHALL explain how this affects sync.

#### Scenario: Valid declaration rejected by the grammar

- **WHEN** a user looks up the diagnostic for `unsafe extern "C" { pub safe fn f(); }`
- **THEN** the documentation identifies the grammar limitation and explains that sync keeps the last good structure and
  exits with status 1
