# format-components Specification

## Purpose

Defines how an installed format plugin describes itself, scans source bytes and returns nodes that Ardent can use to
build an outline.

## Requirements

### Requirement: Versioned scanner interface

A format component SHALL export `ardent:format/scanner@0.1.0` from the `format-plugin` world. Its `describe` function
SHALL return its name, version, default file patterns and search prefixes. A missing or incompatible export SHALL fail
package verification.

#### Scenario: Unsupported interface

- **WHEN** a configured Rust package exports only an unsupported scanner version and the Markdown package is valid
- **THEN** sync names the Rust plugin in its error, retains existing structures, scans changed Markdown files and exits
  with status 1

### Requirement: Deterministic scan results

The scanner SHALL accept source bytes and return either nodes in document order, the first syntax error with a one-based
line and character column, or a failure message. Given the same component and bytes, the result SHALL be the same across
supported platforms and regardless of earlier scans.

#### Scenario: Reuse after a syntax error

- **WHEN** the Rust component scans a valid fixture, an incomplete Rust file, and the same valid fixture again
- **THEN** both successful calls return identical nodes, labels, parents and byte ranges

### Requirement: Validation before outline construction

The host SHALL verify document order, parent indices, tree nesting, parent containment and source bounds before building
the outline. Each label range SHALL fit within its node. A malformed result SHALL fail the file's scan.

#### Scenario: Invalid child range

- **WHEN** a format plugin returns a child whose range extends beyond its parent
- **THEN** sync reports the affected file, preserves its previous structure and stamp if present, continues with other
  files, and exits with status 1

### Requirement: Bounded and isolated execution

The host SHALL limit guest memory and execution, supply deterministic clocks and randomness, and deny ambient
filesystem, network and environment access. Each active format SHALL run in its own instance. After a trap or exhausted
budget, the next scan of that format SHALL start in a fresh instance with a fresh budget.

#### Scenario: Execution budget exhausted

- **WHEN** a component exceeds its budget on one file and can scan the following file within budget
- **THEN** sync reports the first file and leaves its last good structure intact, writes the following file's structure,
  and exits with status 1

#### Scenario: Cold and warm query scans

- **WHEN** a component exports `ardent:format/query-preparation@0.1.0`
- **THEN** the host SHALL call `prepare` once before the first scan in each instance, with a separate execution budget
- **AND** the first file SHALL receive the same scan budget as each subsequent file
