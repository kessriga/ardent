---
id: decision-3
title: >-
  Format plugins are tree-sitter grammars with outline queries, and view plugins
  are WebAssembly components
date: '2026-11-20 00:11'
status: accepted
---
## Context

Format support (Markdown, Rust, HTML) and views (complexity, comments) come from plugins, so Ardent can grow without
changes to its core. The same plugins should run in a future web UI. decision-1 lists the data that a format plugin
supplies for each node. decision-2 adds a name and a version for each format plugin, and asks for room for a later
source-edit operation. M1 ships built-in Markdown and Rust support before a plugin host exists.

Three choices came from the design discussion:

- Wasmtime hosts the plugins through the WebAssembly component model.
- Every format builds on a tree-sitter grammar. A format plugin is a grammar and an outline query, not code.
- Every view comes from plugin code, comments included. A view plugin can attach its own fields to its entries.

Zed is the closest prior art. It runs its extensions as components in Wasmtime, with a WIT interface. It compiles
tree-sitter grammars to WebAssembly, loads them through tree-sitter's `WasmStore`, and finds outline items with an
`outline.scm` query ([Zed extensions][zed-ext], [Zed language support][zed-lang]). doc-1 found the same grammar-loading
path in the tree-sitter Rust crate.

## Decision

### Runtime

Ardent hosts plugins in Wasmtime.

- Format plugins load their grammar through the `wasm` feature of the `tree-sitter` crate: `WasmStore::new(&Engine)` and
  `WasmStore::load_language`.
- View plugins are WebAssembly components. Their interface is the WIT package `ardent:plugin`, version `0.1.0` (see
  "Interface").
- Grammars and view plugins share one Wasmtime `Engine`. `WasmStore::new` takes the `Engine` type of the Wasmtime
  version that `tree-sitter` depends on: 48.0.8 for `tree-sitter` 0.27.2 ([Cargo.toml][ts-cargo]). Ardent therefore uses
  the Wasmtime version that `tree-sitter` pins, not the newest one (50.0.3 on 2026-11-20).
- View plugins target WASI 0.2 (`wasm32-wasip2`). They need no asynchronous I/O, so WASI 0.3 adds nothing for them.
- A plugin gets no files, no network, and no environment. The host fixes the clock and seeds the random source. The
  output of a plugin therefore depends only on its input, on every platform. Labels and fingerprints need this.
- The host limits the memory and the run time of each call. A trap or an exceeded limit is an error result of that call.

A future web UI can reuse both kinds of plugin. web-tree-sitter loads the same grammar `.wasm` files and runs the same
queries. jco transpiles a component to core WebAssembly and JavaScript; its documentation still calls browser support
experimental ([jco][jco]).

### Format plugins

A format plugin is data, not code: a tree-sitter grammar compiled to WebAssembly, an outline query, and a manifest.

```text
markdown/
  plugin.yaml    manifest
  grammar.wasm   output of `tree-sitter build --wasm`
  outline.scm    outline query
```

The manifest gives:

- the plugin name and version, which go into the stamp (decision-2);
- the grammar file and the query file;
- the file patterns that the plugin parses by default, such as `*.md`;
- the search prefixes that the plugin offers, and the files that each prefix covers (decision-1);
- a schema version, so that a later manifest format can add fields without breaking older plugins.

TASK-5 fixes the spellings, where plugins live, and how the project configuration overrides the file patterns.

The stamp of decision-2 also records a hash of the grammar and query files. A changed query therefore re-scans its files
even when the plugin author did not change the version.

#### Outline query

The outline query is a tree-sitter query with two captures and one property:

| Query part          | Meaning                                                                       |
| ------------------- | ----------------------------------------------------------------------------- |
| `@node`             | The node. The byte range of the capture is the range of the node.             |
| `@label`            | One part of the label. A match can have more than one `@label` capture.       |
| `(#set! kind "fn")` | The node kind. Without it, the kind is the tree-sitter type of the `@node`.   |

- Ardent joins the text of all `@label` captures of one match, in document order, with one space. Their byte ranges are
  the label ranges that decision-1 removes before it computes a fingerprint.
- A match without `@label` gets its kind as its label, as decision-1 states for nodes without a natural name.
- The parent of a node is the nearest `@node` whose range contains it. The file is the root node.
- The query only finds nodes, kinds, and labels. Ardent core builds node paths, escapes labels, adds `[n]` suffixes, and
  computes fingerprints (decision-1).

The examples below show the form. TASK-7 and TASK-8 write and test the real queries.

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

#### Source edits

decision-2 asks for room for edits through the outline. A rename replaces the label ranges. A move cuts and inserts node
ranges. Ardent can do both from the outline, without plugin code. To insert a new node, Ardent needs text in the syntax
of the format. A later manifest version can add templates or an optional component for this.

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

- **Labels and fingerprints.** The label ranges are the ranges of the `@label` captures, so the fingerprint of
  decision-1 removes exactly the label text. A label is source text: the Markdown heading `## Sync **rules**` has the
  label `Sync **rules**`.
- **Byte ranges.** tree-sitter gives the byte range of every node, which the structure keeps for later edits
  (decision-2).
- **Syntax errors.** tree-sitter recovers from syntax errors. In the throwaway check, a half-typed `fn helper(` removed
  only that function from the outline; its siblings `fn run` and `fn after` kept their paths. Nodes inside a broken
  region can drop out until the file parses again, and their links then go through the fingerprint search of decision-1.
  tree-sitter marks a tree that contains errors (`Node::has_error`), so Ardent can tell a syntax error from a deleted
  node. The task that implements sync decides how to use this. Without such a rule, a person who types in an editor
  while the TUI runs can turn links into orphans, and each status change rewrites view files.
- **Determinism.** The same grammar, query, and source give the same outline on every platform. A new grammar or query
  changes the stamp (see "Format plugins"), so decision-2 re-scans the files. When the new version changes labels, the
  matcher treats the changes as renames.
- **Markdown setext headings.** `tree-sitter-md` makes sections only for ATX headings, such as `## Title`. A setext
  heading, which is a title underlined with `===` or `---`, gets no section, so the content below it does not nest under
  it. TASK-7 decides how to handle this.

### M1 built-in formats

- The built-in Markdown and Rust plugins have the same shape as external ones: a manifest and an outline query, embedded
  in the binary.
- Their grammars link natively through the `tree-sitter-md` and `tree-sitter-rust` crates instead of loading `.wasm`
  files.
- One outline engine runs the queries of built-in and external plugins. Only grammar loading differs: a native
  `Language` in M1, and `WasmStore::load_language` when the plugin host exists. The built-in plugins can then also ship
  as `.wasm` packages with the same queries.
- M1 needs no Wasmtime and has no view plugins.
- A new crate `ardent-format` holds the outline engine and the built-in plugins. It depends on `ardent-core` and
  `tree-sitter`. `ardent-core` keeps the outline types and gets no parser dependency. The dependency direction in M1 is
  `ardent-core ← ardent-format ← ardent-cli`. A later crate `ardent-plugin-host` adds Wasmtime, grammar loading from
  `.wasm` files, and view plugins, between `ardent-format` and `ardent-cli`.

## Consequences

- A new format needs a tree-sitter grammar and a query, and no code. A format without a tree-sitter grammar needs a
  grammar first.
- A format plugin cannot compute what a query cannot express, such as a label that is not source text. If a format needs
  this, a later manifest version can add an optional component.
- The Wasmtime version of Ardent follows the pin of `tree-sitter`. A Wasmtime upgrade waits for a `tree-sitter` release.
- A plugin cannot read files or use the network. A view that needs outside data, such as test coverage, needs a new host
  function first.
- This record refines decision-2: the view files in git hold stored views only. Derived views live in the derived part
  of `.ardent/`, next to the structure.
- TASK-5 must fix the manifest spellings and where plugins live, how the project configuration enables plugins and
  overrides file patterns, where plugin fields go in the YAML of an entry, and where derived view output lives.
- A web UI needs the outline engine in the browser, for example `ardent-format` compiled to WebAssembly on top of
  web-tree-sitter. The grammars, the queries, and the view components carry over.
- The WIT package version changes when the interface changes. The host can support more than one version at a time.

[zed-ext]: https://zed.dev/blog/zed-decoded-extensions
[zed-lang]: https://zed.dev/blog/language-extensions-part-1
[ts-cargo]: https://docs.rs/crate/tree-sitter/0.27.2/source/Cargo.toml.orig
[jco]: https://github.com/bytecodealliance/jco
