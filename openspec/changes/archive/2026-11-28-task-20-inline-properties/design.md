# Design

## Context

Decision-8 gives format plugins ownership of source parsing and leaves node paths and fingerprints to Ardent. The CLI
already loads a structure, verifies the source hash, and asks the matching format component for optional highlighting.
The terminal browser receives that document and tracks selection and child expansion separately from source scrolling.

## Goals / Non-Goals

Give the terminal browser an ordered set of displayable properties for each node. Keep the plugin’s choice of inline
visibility separate from whether a property exists. The full-property modal and the view plugin interface for stored
properties are outside this change.

## Decisions

### Load source properties with the open document

Use an optional format component export alongside the scanner, following the highlighter precedent. The proposed call
takes the source bytes and returns groups of named display values in plugin order. Each group identifies the node by its
zero-based position in the scanner result, before the host adds the file root. Include the inline choice on each
property. A missing opt-in means hidden.

The guest must derive these positions from the same ordered nodes it supplies to the scanner. A list and its only item
can cover the same byte range, so matching by range alone could put a checkbox on its list. Bind the result to the
component hash and source bytes used for the open outline. On refresh, replace the values with those for that revision.

In `ardent-format`, bound the call and validate the groups before exposing them to the CLI. Reject invalid indices and
ambiguous duplicate groups or property names. A missing export, trap or malformed result leaves the document readable.
Keep the plain property data in `ardent-core` and pass it through `ardent-cli` to the document in `ardent-tui`.

Extending the required scanner record would break existing components. Persisting the properties in the structure would
also need a cache format decision. Loading properties once per open document costs an additional guest call but fits the
existing browser load and refresh paths.

### Take values from each declaration

The Markdown guest reads the task marker directly on the list item. The Rust guest uses the declaration’s own syntax for
visibility, type and inline generic bounds. A type parameter’s bound describes a constraint. A const parameter’s type
describes its value. The guest emits the corresponding property name and keeps source qualifiers such as `pub(crate)`.

Create signature children during the Rust scan so sync, node paths, keyboard navigation and source selection all see the
same tree. Scope the matches to the function’s own signature, including trait and extern signatures. A function-pointer
type nested inside an input must not contribute extra signature children to the enclosing function. Use the written
binding or pattern as the input label, and `self` for all receivers. Give the return child the span from the arrow
through the type. Use `return` as the node kind and its fallback label. Decision-10 gives this generic node the path
segment `return.1` and the display label `return` when it is the only such child. The existing path and fingerprint
rules in decision-1 still apply.

### Keep row display separate from node navigation

Store property disclosure state separately from branch expansion in the open document. Restore each surviving node’s
disclosure by node path on refresh, as the browser does for branches. The renderer inserts property rows immediately
after the owning node. Count those rows when positioning the viewport so moving Down past a long block keeps the next
selected node visible. Use the browser’s safe terminal text handling for both names and values.

The tree continues to navigate nodes. This keeps the current preview, source scroll and selection when properties open
or close. Adding synthetic child nodes for properties would give them navigation stops and node paths. Shift+Tab remains
available in either focused pane while a document is open.

## Risks / Trade-offs

- A plugin update can change Rust signature children without a source edit. The existing format hash in the sync stamp
  triggers a rescan. Cover the transition from an old installed package.
- Only visible rows that fit can be drawn on a short terminal. Cover the selected node and the first row after an open
  property block in narrow-terminal rendering tests.
- Signature inputs may contain nested types, attributes or patterns. Use source-range and node-path assertions where a
  snapshot of the labels alone could pass with the wrong selection.

## Migration Plan

Rebuild the bundled Markdown and Rust packages. Projects pick up the new guests through the existing `ardent update`
flow. Older scanner-only packages remain usable. The design requires no change to stored view files or the structure
schema.
