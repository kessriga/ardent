# Rust format

## ADDED Requirements

### Requirement: Applicable declaration properties

The Rust format plugin SHALL opt a declaration’s written visibility into inline display as `visibility`. It SHALL expose
the declared type on statics, constants and fields, and the assigned type on a type alias, as inline `type`. It SHALL
omit absent or inapplicable values. A function’s return type belongs to its return child.

#### Scenario: Public static and private field

- **WHEN** the reader opens properties on `pub static EAST_FIELD: Gauge = Gauge::new();` and on a private field
  `count: usize`
- **THEN** `static EAST_FIELD` shows `visibility: pub` and `type: Gauge`, and `count` shows only `type: usize`

#### Scenario: Restricted visibility and inherited visibility

- **WHEN** the reader inspects `pub(crate) enum State { Ready }`
- **THEN** `enum State` shows `visibility: pub(crate)` and its variant `Ready` has no visibility property

### Requirement: Selectable function signature children

A function definition or signature SHALL have a child for each of its own generic parameters and inputs, in declaration
order, followed by a child labeled `return` if it has a written return clause. Selecting a child SHALL preview its
declaration, including the arrow for a return clause. Each child SHALL have a node path under the function and use the
outline’s existing sibling naming rules.

#### Scenario: Inspect a generic function

- **WHEN** the reader expands `pub fn find<T: Lookup>(key: &str) -> Option<T> { todo!() }`
- **THEN** the signature children are `T`, `key` and `return` in that order
- **AND** they preview `T: Lookup`, `key: &str` and `-> Option<T>` respectively
- **AND** their inline properties are `bound: Lookup`, `type: &str` and `type: Option<T>` respectively
- **AND** the function itself shows `visibility: pub` and has no `type` property

#### Scenario: Explicit unit return

- **WHEN** a trait declares `fn first() -> ();` and `fn second();`
- **THEN** only `fn first` has a `return` child and that child shows `type: ()`

### Requirement: Written properties of signature parameters

The Rust format plugin SHALL opt each input’s written type into inline display, including an explicit receiver type. It
SHALL label a receiver `self` and omit its type property for shorthand forms. A const generic SHALL show its declared
`type`. A type or lifetime parameter with bounds written on the parameter SHALL show them as `bound`.

#### Scenario: Compare receivers

- **WHEN** the reader inspects methods declared with `&mut self` and `self: Box<Self>`
- **THEN** each has a `self` child that previews the entire receiver declaration
- **AND** only the second receiver shows `type: Box<Self>`

#### Scenario: Lifetime and const generic

- **WHEN** a function declares `<'a, const N: usize>`
- **THEN** its signature contains selectable children labeled `'a` and `N`
- **AND** `'a` has no inline properties and `N` shows `type: usize`
