# outline-browser changes

## RENAMED Requirements

- FROM: `### Requirement: Number items within each displayed list`
- TO: `### Requirement: Display sibling positions`

## MODIFIED Requirements

### Requirement: Display sibling positions

The browser SHALL display the one-based dotted position for every sibling in a repeated label group across all formats.
A singleton SHALL display its original label.

#### Scenario: Read a nested list

- **WHEN** a reader expands a two-item list whose first item contains a one-item list
- **THEN** the outer list shows `item.1` and `item.2`, and the inner list shows `item`
- **AND** selecting each row shows the source for its node path

#### Scenario: Read repeated names

- **WHEN** two sibling headings are named Install and another is named Usage
- **THEN** the outline displays `Install.1`, `Install.2`, and `Usage`
