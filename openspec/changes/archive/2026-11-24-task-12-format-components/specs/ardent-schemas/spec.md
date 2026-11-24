# Spec Delta

## MODIFIED Requirements

### Requirement: Format hash

For every format plugin, `format_hash` SHALL be the BLAKE3 hash of the complete `format.wasm` file, as 64 lowercase hex
digits.

#### Scenario: Changed query

- **WHEN** a person installs a rebuilt `.ardent/plugins/html/format.wasm` with a changed query and the same manifest
  version
- **THEN** the next sync finds a different `format_hash` and scans every tracked HTML file again
