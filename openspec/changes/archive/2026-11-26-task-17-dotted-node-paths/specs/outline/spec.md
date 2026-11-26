# outline changes

## MODIFIED Requirements

### Requirement: Node path

The node path of a node SHALL be the source path, `#`, and the labels of the nodes from the root down to the node,
joined by `/`. A backslash SHALL escape each `.`, `/`, `#`, `[`, `]`, and `\` in a label, and each `#`, `[`, `]`, and
`\` in the source path.

#### Scenario: Nested heading

- **WHEN** the heading `Renames` is a child of the heading `Sync rules` in `docs/spec.md`
- **THEN** its node path is `docs/spec.md#Sync rules/Renames`

#### Scenario: Slash in a label

- **WHEN** `docs/spec.md` has the heading `## Read/write`
- **THEN** its node path is `docs/spec.md#Read\/write`

#### Scenario: Hash sign in a source path

- **WHEN** the tracked file `docs/C#.md` has the heading `## Intro`
- **THEN** its node path is `docs/C\#.md#Intro`

### Requirement: Positional suffixes

A generic label (the node kind with no nonblank source label) SHALL always have a one-based dotted position. A named
node SHALL have a position only when siblings share its label. Positions SHALL count siblings with the same original
label in document order and start again under each parent.

#### Scenario: Two paragraphs

- **WHEN** the heading `Sync rules` in `docs/spec.md` has two paragraphs as children
- **THEN** their node paths are `docs/spec.md#Sync rules/paragraph.1` and `docs/spec.md#Sync rules/paragraph.2`

#### Scenario: Two headings with one name

- **WHEN** `README.md` has two headings `## Install` with the same parent
- **THEN** their node paths are `README.md#Install.1` and `README.md#Install.2`


#### Scenario: Literal dotted name

- **WHEN** two headings named Install share a parent with a heading named Install.1
- **THEN** their path segments are `Install.1`, `Install.2`, and `Install\.1` respectively
