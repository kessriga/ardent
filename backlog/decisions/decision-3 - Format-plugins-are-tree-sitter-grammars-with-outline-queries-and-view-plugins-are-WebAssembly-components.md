---
id: decision-3
title: >-
  Format plugins are tree-sitter grammars with outline queries, and view plugins
  are WebAssembly components
date: '2026-11-20 00:11'
status: partially superseded by decision-8
---
## Context

Decision-8 replaces the original format-plugin design. The view plugin interface and the host rules below still apply.

Format support (Markdown, Rust, HTML) and views (complexity, comments) come from plugins, so Ardent can grow without
changes to its core. The same plugins should run in a future web UI. decision-1 lists the data that a format plugin
supplies for each node. decision-2 adds a name and a version for each format plugin, and asks for room for a later
source-edit operation. M1 ships built-in Markdown and Rust support before a plugin host exists.

The original design discussion made three choices:

- Wasmtime hosts the plugins through the WebAssembly component model.
- Every format builds on a tree-sitter grammar. A format plugin is a grammar and an outline query, not code.
- Every view comes from plugin code, comments included. A view plugin can attach its own fields to its entries.

Zed is the closest prior art. It runs its extensions as components in Wasmtime, with a WIT interface. It compiles
tree-sitter grammars to WebAssembly, loads them through tree-sitter's `WasmStore`, and finds outline items with an
`outline.scm` query ([Zed extensions][zed-ext], [Zed language support][zed-lang]). doc-1 found the same grammar-loading
path in the tree-sitter Rust crate.

## Decision

### Runtime

Ardent hosts format and view plugins in Wasmtime as WASI 0.2 components. Decision-8 defines the format interface. The
view interface uses the WIT package `ardent:plugin@0.1.0`. The host limits memory and execution time, fixes clocks and
randomness, and denies ambient access to files, the network and the environment.

### Format plugins

Decision-8 defines the component, its manifest, its outline engine, and the format hash used in a sync. The `outline`
spec defines the query rules. Ardent core adds the root and builds node paths and fingerprints from the returned nodes.

The following example queries are used by the structure-file examples in the `ardent-schemas` spec. The release plugins
have their own queries.

```scheme
; Rust
((mod_item "mod" @label name: (identifier) @label) @node
  (#set! kind "mod"))

((function_item "fn" @label name: (identifier) @label) @node
  (#set! kind "fn"))

((impl_item "impl" @label type: (_) @label) @node
  (#set! kind "impl"))
```

```scheme
; Markdown
((section (atx_heading heading_content: (inline) @label)) @node
  (#set! kind "heading"))

((paragraph) @node
  (#set! kind "paragraph"))
```

A throwaway check with `tree-sitter` 0.27.2, `tree-sitter-rust` 0.24.2, and `tree-sitter-md` 0.5.4 ran these queries
with the parent rule above. It gave the expected paths, such as `mod engine/impl Engine/fn run` and
`Spec/Sync rules/Renames`.

For future source edits, a rename uses the label ranges and a move uses the node range. Inserting an item requires text
in the format's syntax.

### View plugins

Every view comes from a view plugin. A view plugin is a WebAssembly component in one of two worlds:

| World          | Who writes the entries                                       | Examples              | Ardent stores them in                       |
| -------------- | ------------------------------------------------------------ | --------------------- | ------------------------------------------- |
| `stored-view`  | People, agents, and the CLI.                                 | comments              | view files in git (decision-2)              |
| `derived-view` | The plugin computes them from a source file and its outline. | complexity, backlinks | the derived part of `.ardent/`, not in git |

#### Entries

An entry has a host part and a plugin part.

- The host part is the anchor (decision-1) and the `refs` list. Only Ardent writes them.
- The plugin part is a list of named fields. A field value is a text, a number, a flag, or a list of texts. WIT has no
  recursive types, so fields stay flat in version `0.1.0`. Ardent stores the fields as YAML next to the anchor and the
  `refs` list. The `text` in the example of decision-1 becomes a field of the comment plugin.

A derived entry addresses its node by node path at compute time, with no fingerprint and no status. Ardent recomputes
derived entries after each re-scan, so they never become orphans.

#### References

Ardent searches every text value for references in the form `@[<node path>]` (decision-1). It resolves them, writes the
`refs` list, and rewrites the paths in the text when a node moves. A plugin writes a reference when it puts
`@[<node path>]` into a text field. It never parses or resolves references itself. References are therefore values that
the host understands, not private plugin data.

The backlink index covers stored entries only. Derived entries can contain references, and the TUI can follow them, but
the index ignores them. A backlink view therefore never indexes its own output, and Ardent can rebuild the index from
the view files in git.

#### Calls

- **Stored views.** Ardent calls `check` when a person, an agent, or the CLI adds or changes an entry. A sync also calls
  `check` for each entry of a view file that changed outside Ardent (decision-2). `check` returns the normalized fields
  or an error. When an entry fails `check` during a sync, Ardent keeps the entry unchanged and reports the error. It
  never deletes the entry.
- **Derived views.** Ardent calls `compute` for each changed source file after the re-scans of a sync, and stores the
  result in the derived part. Ardent also recomputes the entries of a file when a host function result that the last
  `compute` read has changed. The implementing task decides how precisely Ardent tracks this.
- **Both.** `summary` gives the line that the outline shows for an entry. The line can contain references.
- **Host functions.** Every view plugin can call `outline-of`, which returns the current outline of a tracked file, and
  `backlinks`, which returns the stored entries, in every view of every file, with a reference to a node. A backlink
  view is a derived view that calls `backlinks` for each node of its file.

#### Interface

The WIT below is a draft. The task that builds the plugin host commits it and may change the details. A throwaway check
parsed it with `wit-parser` 0.263.0.

```wit
package ardent:plugin@0.1.0;

interface types {
  /// Byte offsets into a source file. `end` is exclusive.
  record span {
    start: u32,
    end: u32,
  }

  /// A node path as decision-1 defines it.
  type node-path = string;

  /// `parent` is an index into `outline.nodes`. The root has no parent.
  record node {
    path: node-path,
    kind: string,
    label: string,
    range: span,
    parent: option<u32>,
  }

  /// The nodes of one source file in document order, root first.
  record outline {
    file: string,
    nodes: list<node>,
  }

  variant value {
    text(string),
    number(f64),
    flag(bool),
    texts(list<string>),
  }

  record field {
    name: string,
    value: value,
  }

  /// A stored entry with a reference to the target node.
  record backlink {
    view: string,
    anchor: node-path,
    summary: string,
  }

  record plugin-info {
    name: string,
    version: string,
  }
}

interface host {
  use types.{node-path, outline, backlink};

  /// The current outline of a tracked file.
  outline-of: func(file: string) -> option<outline>;

  /// The stored entries, in every view of every file, with a reference to `target`.
  backlinks: func(target: node-path) -> list<backlink>;
}

interface stored {
  use types.{field, plugin-info};

  info: func() -> plugin-info;

  /// Validates and normalizes the fields of a new or changed entry.
  check: func(fields: list<field>) -> result<list<field>, string>;

  /// The line that the outline shows for the entry. It can contain references.
  summary: func(fields: list<field>) -> string;
}

interface derived {
  use types.{field, node-path, outline, plugin-info};

  record entry {
    node: node-path,
    fields: list<field>,
  }

  info: func() -> plugin-info;

  /// Computes the entries for one source file.
  compute: func(outline: outline, source: list<u8>) -> result<list<entry>, string>;

  /// The line that the outline shows for the entry. It can contain references.
  summary: func(fields: list<field>) -> string;
}

world stored-view {
  import host;
  export stored;
}

world derived-view {
  import host;
  export derived;
}
```

### Effect on anchors

The host uses the label ranges from each node to compute its fingerprint. The component supplies byte ranges for later
edits. Decision-5 governs heading levels. Decision-7 governs syntax errors, so a failed scan keeps the last good
structure and stamp.

### Distribution

Decision-8 replaces the M1 built-in distribution. The release ships Markdown and Rust as separate components. TASK-12
migrates scanning, and TASK-13 installs the selected plugins during initialization.

## Consequences

- A plugin cannot read files or use the network. A view that needs outside data, such as test coverage, needs a new host
  function first.
- This record refines decision-2: the view files in git hold stored views only. Derived views live in the derived part
  of `.ardent/`, next to the structure.
- TASK-5 must fix the manifest spellings and where plugins live, how the project configuration enables plugins and
  overrides file patterns, where plugin fields go in the YAML of an entry, and where derived view output lives.
- The WIT package version changes when the interface changes. The host can support more than one version at a time.

[zed-ext]: https://zed.dev/blog/zed-decoded-extensions
[zed-lang]: https://zed.dev/blog/language-extensions-part-1
