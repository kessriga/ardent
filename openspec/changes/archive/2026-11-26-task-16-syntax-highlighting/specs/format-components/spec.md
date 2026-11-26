# Format components

## ADDED Requirements

### Requirement: Optional source highlighting

A format plugin MAY export a highlighter alongside the required scanner. The highlighter SHALL identify syntax
categories using ordered, disjoint, nonempty half-open byte ranges in the supplied source. The host SHALL bound the call
and validate the result before using any ranges.

#### Scenario: A scanner-only plugin

- **WHEN** a reader opens a file whose installed format plugin supplies only the scanner
- **THEN** the reader sees the plain source and can select nodes in its outline

#### Scenario: An unusable highlight result

- **WHEN** an otherwise valid scanner returns overlapping or out-of-bounds highlight ranges, or its highlight call traps
- **THEN** the reader sees plain source and can navigate the outline
- **AND** the file keeps its successful scan result
