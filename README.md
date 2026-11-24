# Ardent

Ardent is a terminal outliner for structured text. It reads the files in a project, such as Markdown, Rust, or HTML, and
shows each file as a tree of nodes, in the same outline form that Roam Research and Tana use.

> **Status:** milestone M0 has settled the core design, and milestone M1 builds a read-only outline. The binary has two
> commands: `ardent init` and `ardent sync`. The current binary embeds the Markdown and Rust format plugins. TASK-12
> moves them into components, and TASK-13 adds plugin selection to `ardent init`. Run `backlog milestone list` to see
> progress.

## Idea

Most outliners own their content: you write inside the tool, and the tool stores the tree. Ardent works the other way
round. The source files stay the source of truth, and Ardent derives an outline from their existing structure:

- A Markdown file becomes a tree of headings, lists, and paragraphs.
- A Rust file becomes a tree of modules, items, and function bodies.
- An HTML file becomes a tree of elements.

On top of that outline, Ardent keeps **views**: extra data attached to nodes. Examples are the cyclomatic complexity of
each function, or review comments from a team about a particular function. A view never changes the source file.

A structured, hierarchical picture of a project helps people. It should also help AI agents, which work better when they
can address a function or a section by its place in a tree rather than by line numbers.

## How it should work

Run `ardent init` in the root directory of a project. It creates a `.ardent/` directory with a `config.yaml` and a
`.gitignore`. It refuses to run where `.ardent` already exists or inside another Ardent project. The configuration
selects the plugins and the files that Ardent tracks. In the component design, `ardent init` installs the plugins you
select from the release. For each tracked file, Ardent keeps:

- the **structure** of the file, as human-readable YAML;
- zero or more **views** of the file, also as YAML.

`.ardent/` has two parts:

- **User data**, in git: `config.yaml`, project plugins in `plugins/`, and stored views in `views/`, such as the
  comments that people and agents write. The comments on `src/engine.rs` are in
  `.ardent/views/src/engine.rs/comments.yaml`.
- **Derived data**, in `cache/`, which git ignores: the structure files, the views that plugins compute, and the
  backlink index. When the cache is missing, Ardent rebuilds it from the source files and `views/`.

Run `ardent sync` anywhere in the project. It scans each tracked file that changed since the last sync and writes its
structure to `.ardent/cache/files/<source path>/structure.yaml`. It skips a file whose size, modification time, and
format plugin are unchanged, and it deletes the structure of a file that Ardent no longer tracks. Ardent follows git's
ignore rules, and it skips a subdirectory that holds its own `.ardent/` as a separate project. When a file has a syntax
error, `ardent sync` reports the file with the line and column of the error, keeps the structure from its last scan, and
exits with status 1.

The outline of a Markdown file holds headings, paragraphs, lists and their items, code blocks, block quotes, tables, and
front matter. A heading contains the blocks after it, up to the next heading of the same or a higher rank: `## Usage`
ends a `### Details` section. The text of a list item is part of the item, so its paragraphs are not separate nodes.

The outline of a Rust file holds its items and their members. Items are modules, impl blocks, traits, functions,
structs, enums, unions, constants, statics, type aliases, `macro_rules!` macros, and extern blocks. Members are fields,
enum variants, and the functions, constants, and types of traits and impl blocks. The label of an item is its keyword
and its name, such as `fn run` or `impl Display for Engine`. The doc comments and attributes above an item are part of
its node. `use` declarations, macro calls, and the statements in a function body are not nodes.

Format support (how to parse Rust, Markdown, and so on) and views (complexity, comments) come from **plugins**. In the
component design, a format plugin receives source bytes and returns outline nodes or an error position. Query-based
plugins package tree-sitter, a grammar, and the outline query together. A view plugin stores or computes view entries.

### Example: comments and backlinks

A developer uses the comment plugin to leave a comment on a Rust function: "This function contradicts @foo and also
function @bar."

- The comment is a view entry. Its anchor attaches it to the function.
- `@foo` is a reference to one section of a Markdown spec document.
- `@bar` is a reference to a function in a different source file.

When someone opens that spec section or the function `bar`, the backlink plugin shows the comment as a backlink. Ardent
therefore needs one addressing scheme that covers every node in the project, a reference syntax that a person can type,
and an index from each node to the references that point at it.

## Design decisions

Run `backlog decision list` to find the design decisions.

- Decided in decision-1: anchors and references address a node by its node path, such as
  `docs/spec.md#Sync rules/Renames`, and keep a fingerprint of its content to find it again after a rename or move. A
  reference in view content has the form `@[docs/spec.md#Sync rules]`.
- Decided in decision-2: every Ardent command first checks the tracked files for changes and re-scans the changed ones.
  The structure is a local cache; only stored views, the project configuration, and project plugins go into git.
- Decided in decision-4: the layout of `.ardent/` and the YAML schema of each file in it. Every file starts with a
  `schema` version. The specs `ardent-layout` and `ardent-schemas` in `openspec/specs/` give the details.
- Decision-3 defines view plugins. Ardent finds and rewrites references in their data. Decision-8 defines format plugins
  as WebAssembly components. Ardent uses the manifest and component hash to check stamps before starting Wasmtime.
- Decided in decision-5, during M1: an outline query can give a node a `level`, and the node then contains the siblings
  after it up to the next one with the same or a smaller level. Markdown headings nest this way.

## Glossary

Use these terms with these meanings in code, specs, and tasks.

| Term          | Meaning                                                                                   |
| ------------- | ----------------------------------------------------------------------------------------- |
| source file   | A file in the user's project. Ardent reads it; the user owns it.                          |
| node          | One element of a source file's structure, such as a heading or a function.                |
| outline       | The tree of nodes for one source file.                                                    |
| node path     | The address of a node: file path, `#`, and node labels joined by `/`.                     |
| fingerprint   | A hash of a node's content without its label. It finds a node again after a rename.       |
| structure     | The stored form of an outline in `.ardent/`. A local cache that Ardent rebuilds.          |
| sync          | Re-scans changed source files and re-resolves links. Every command runs it first.         |
| view          | Extra data attached to nodes of an outline. A view plugin stores or computes it.          |
| view entry    | One item of a view, such as one comment. Its anchor attaches it to a node.                |
| anchor        | The link that attaches a view entry to a node or a range of a source file.                |
| reference     | A link written inside view content to a node anywhere in the project, such as `@foo`.     |
| backlink      | A reference seen from its target: the list of view entries that point at a node.          |
| format plugin | A plugin that scans files of one format and returns nodes for their outlines.             |
| view plugin   | A WebAssembly component that stores or computes a view, such as comments or complexity.   |

## Building

```sh
cargo build
cargo run -p ardent-cli   # the `ardent` binary
```

The workspace has three crates. `ardent-core` holds the document model: outlines, structure files, and the
configuration. `ardent-format` holds the outline engine and the built-in format plugins, and depends on `ardent-core`.
`ardent-cli` builds the `ardent` binary and depends on both.
