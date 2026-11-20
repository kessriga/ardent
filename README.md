# Ardent

Ardent is a terminal outliner for structured text. It reads the files in a project, such as Markdown, Rust, or HTML, and
shows each file as a tree of nodes, in the same outline form that Roam Research and Tana use.

> **Status:** design phase. The binary does nothing yet. The open design questions are tasks in milestone M0; run
> `backlog milestone list` to see progress.

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

Running `ardent init` in a project creates a `.ardent/` directory. For each tracked file, that directory holds:

- the **structure** of the file, as human-readable YAML;
- zero or more **views** of the file, also as YAML.

Format support (how to parse Rust, Markdown, and so on) and views (complexity, comments) come from **plugins**. A format
plugin is a tree-sitter grammar and a query that finds the nodes of the outline. A view plugin is a WebAssembly
component that stores or computes view entries. The same plugins can run in a future web UI.

### Example: comments and backlinks

A developer uses the comment plugin to leave a comment on a Rust function: "This function contradicts @foo and also
function @bar."

- The comment is a view entry. Its anchor attaches it to the function.
- `@foo` is a reference to one section of a Markdown spec document.
- `@bar` is a reference to a function in a different source file.

When someone opens that spec section or the function `bar`, the backlink plugin shows the comment as a backlink. Ardent
therefore needs one addressing scheme that covers every node in the project, a reference syntax that a person can type,
and an index from each node to the references that point at it.

## Open questions

These questions decide the design. Each one is a spike task in milestone M0.

- Decided in decision-1: anchors and references address a node by its node path, such as
  `docs/spec.md#Sync rules/Renames`, and keep a fingerprint of its content to find it again after a rename or move. A
  reference in view content has the form `@[docs/spec.md#Sync rules]`.
- Decided in decision-2: every Ardent command first checks the tracked files for changes and re-scans the changed ones.
  The structure is a local cache; only the views and the project configuration go into git.
- What is the exact layout of `.ardent/` and the YAML schema for structure and views?
- Decided in decision-3: a format plugin is a tree-sitter grammar with an outline query, and a view plugin is a
  WebAssembly component that Wasmtime hosts. Ardent finds and rewrites the references in plugin data itself.

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
| view          | Extra data attached to nodes of an outline, stored next to the structure.                 |
| anchor        | The link that attaches a view entry to a node or a range of a source file.                |
| reference     | A link written inside view content to a node anywhere in the project, such as `@foo`.     |
| backlink      | A reference seen from its target: the list of view entries that point at a node.          |
| format plugin | A tree-sitter grammar and an outline query that turn files of one format into outlines.   |
| view plugin   | A WebAssembly component that stores or computes a view, such as comments or complexity.   |

## Building

```sh
cargo build
cargo run -p ardent-cli   # the `ardent` binary
```

The workspace has two crates. `ardent-core` holds the document model. `ardent-cli` builds the `ardent` binary and
depends on `ardent-core`.
