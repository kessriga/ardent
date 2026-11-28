# Format components

## ADDED Requirements

### Requirement: Plugin-owned inline properties

A format plugin MAY supply ordered, named property values for nodes in its outline. The host SHALL show a property
inline only when the plugin explicitly opts that property in. It SHALL associate each value with its node in the
matching source revision. A plugin that exports only the supported scanner SHALL remain usable.

#### Scenario: A plugin supplies two properties

- **WHEN** a plugin supplies two named values for a node and opts only the second into inline display
- **THEN** opening that node’s properties shows only the second value, with its supplied name

#### Scenario: A scanner-only plugin

- **WHEN** the reader opens a file with an installed scanner-only plugin
- **THEN** its outline and preview remain available and Tab adds no property rows

#### Scenario: Unusable properties

- **WHEN** a property call fails or returns a value for a node outside the matching outline
- **THEN** the browser discards the property result for the file and keeps its outline and preview available
