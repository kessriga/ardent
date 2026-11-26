# Outline browser changes

## ADDED Requirements

### Requirement: Open the browser by default

Running `ardent` without arguments SHALL open the same terminal browser as `ardent browse`. Running `ardent --help`
SHALL print the command list and exit successfully.

#### Scenario: Open an initialized project

- **WHEN** a reader runs `ardent` in an initialized project in an interactive terminal
- **THEN** the browser opens the explorer

#### Scenario: Discover commands

- **WHEN** a reader runs `ardent --help` outside a project with redirected output
- **THEN** the output lists the init, sync and browse commands

### Requirement: Distinguish outline and source panes

The browser SHALL draw an unlabeled horizontal divider across the document pane between the outline and source preview
when there is room for both panes and the divider. The divider SHALL leave the explorer and source paging controls
usable.

#### Scenario: Read an outline

- **WHEN** a reader opens a document beside the explorer
- **THEN** the reader sees the source below a horizontal divider that spans only the document pane
- **AND** the divider remains visible when the reader switches to the document in a narrow terminal with room for the
  divider

### Requirement: Number items within each displayed list

The browser SHALL label the items of a multi-item list in document order as `item[1]`, `item[2]` and so on. It SHALL
display a lone item as `item` and apply this rule independently to each nested list.

#### Scenario: Read a nested list

- **WHEN** a reader expands a two-item list whose first item contains a one-item list
- **THEN** the outer list shows `item[1]` and `item[2]`, and the inner list shows `item`
- **AND** selecting each row shows the source range for its existing node path
