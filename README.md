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

Format support (how to parse Rust, Markdown, and so on) and views (complexity, comments) come from **plugins**.
WebAssembly is the likely plugin format, because the same plugins could then run in a future web UI.

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

- How do anchors and references point at a node, in the same source file or another one, so that the link survives
  edits? Byte offsets, tree paths, content hashes, and stable IDs are all candidates.
- How do the source file and its structure stay in sync when someone edits the source outside Ardent?
- What is the exact layout of `.ardent/` and the YAML schema for structure and views?
- What does a plugin export, and which WebAssembly runtime hosts it?

## Glossary

Use these terms with these meanings in code, specs, and tasks.

| Term          | Meaning                                                                                   |
| ------------- | ----------------------------------------------------------------------------------------- |
| source file   | A file in the user's project. Ardent reads it; the user owns it.                          |
| node          | One element of a source file's structure, such as a heading or a function.                |
| outline       | The tree of nodes for one source file.                                                    |
| structure     | The stored form of an outline in `.ardent/`.                                              |
| view          | Extra data attached to nodes of an outline, stored next to the structure.                 |
| anchor        | The link that attaches a view entry to a node or a range of a source file.                |
| reference     | A link written inside view content to a node anywhere in the project, such as `@foo`.     |
| backlink      | A reference seen from its target: the list of view entries that point at a node.          |
| format plugin | A plugin that turns a source file of one format into an outline.                          |
| view plugin   | A plugin that computes a view, for example cyclomatic complexity.                         |

## Building

```sh
cargo build
cargo run -p ardent-cli   # the `ardent` binary
```

The workspace has two crates. `ardent-core` holds the document model. `ardent-cli` builds the `ardent` binary and
depends on `ardent-core`.
