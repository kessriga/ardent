# Markdown format

## ADDED Requirements

### Requirement: Checkbox state on list items

The Markdown format plugin SHALL opt `checked` into inline display on a list item with its own task list marker. A
checked marker SHALL give the value `true` and an unchecked marker SHALL give `false`. The plugin SHALL omit the
property on plain list items.

#### Scenario: Compare checkbox items

- **WHEN** sibling items start with `- [x] Done` and `- [ ] Pending` and the reader opens properties on both
- **THEN** `item.1` shows `⚬ checked: true` and `item.2` shows `⚬ checked: false`

#### Scenario: Plain parent with a checked child

- **WHEN** a plain item contains a nested list with a checked item
- **THEN** only the checked item supplies `checked`, and opening properties on its parent adds no row
