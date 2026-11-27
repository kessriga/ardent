# Ardent

Ardent is a terminal outliner for structured text. It reads the files in a project, such as Markdown, Rust, or HTML, and
shows each file as a tree of nodes, in the same outline form that Roam Research and Tana use.

> **Status:** The CLI provides `ardent init`, `ardent sync`, and `ardent browse`. Markdown and Rust ship as separate
> WebAssembly components. Install the formats your project uses. Run `backlog milestone list` for milestone progress.

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

Keep the release's `plugins/` directory beside the `ardent` executable. Run `ardent init` in the project root. Use the
arrow keys to move between Markdown and Rust, Space to toggle each format, and Enter to confirm. Press q or Ctrl-C to
cancel. Init copies the selected packages into `.ardent/plugins/` and enables them in `.ardent/config.yaml`. It refuses
to run where `.ardent` already exists or inside another Ardent project.

In MSYS and Cygwin terminals that use pipes, enter the format names separated by commas at the prompt.

In a script, pass the format names explicitly. Use `--plugins ''` to start with an empty selection.

```sh
ardent init --plugins markdown,rust && ardent sync
```

The browser names project-installed plugins that differ from the running release. The notice clears after you update
them. Run `ardent update` from anywhere in a project to replace installed Markdown and Rust format plugins with the
copies bundled with the running executable. This includes disabled plugins. The command replaces the whole package,
including local edits and extra files in a fork installed under the same name. A separately named fork keeps its files.
Project settings and the selection of installed plugins stay as they are.

In a shared project, Ardent preserves access rules for existing package files and directories. On Unix, an update that
cannot retain an existing owner leaves that plugin in place. Run the update with an account that can preserve ownership
throughout the package.

If one package fails, Ardent names it and reports which other packages it updated. On filesystems that require two
renames, Ardent temporarily saves the previous package in `.ardent/plugins/.ardent-update-<name>/`. After an
interruption, run `ardent sync` or `ardent update` to recover. Recovery also works after you clear the cache. If the
installed directory contains files Ardent cannot identify as the staged replacement, it leaves both copies in place and
prints the backup path. Move the installed directory aside, then run `ardent sync` to restore the backup. Other formats
remain available while you resolve the conflict.

When upgrading an existing project, make sure each format listed in `.ardent/config.yaml` has a package in the project
or your user configuration before the first sync. Sync prunes cached structures for files it no longer tracks. For
example, if both packages are missing, set `ARDENT_RELEASE` to the directory that contains the release executable and
run these commands from the project root.

```sh
mkdir -p .ardent/plugins &&
  cp -R "$ARDENT_RELEASE/plugins/markdown" "$ARDENT_RELEASE/plugins/rust" .ardent/plugins/ &&
  ardent sync
```

Commit the installed packages with the project. Each includes a `plugin.yaml` manifest and a `format.wasm` component.
Sync warns for each configured format whose package is missing. If a package fails verification, sync reports the
format, keeps existing structures, scans files for healthy formats and exits with status 1.

### Share format plugins and settings

To use one Markdown installation across projects, copy the release's complete `plugins/markdown/` directory to
`$XDG_CONFIG_HOME/ardent/plugins/markdown/`. The configuration home defaults to `~/.config`. Enable Markdown in each
project's `.ardent/config.yaml`:

```yaml
schema: 1
formats:
  markdown: {}
```

A package in `.ardent/plugins/markdown/` pins that project to its own copy. Removing the pin selects the user copy on
the next sync. If the pin fails verification, sync reports the error and keeps the last good structure. To start a new
project with shared packages, run `ardent init --plugins ''` and add the names you want under `formats`. Manage user
packages by copying their release directories. `ardent update` updates project installations.

Put shared settings in the optional `$XDG_CONFIG_HOME/ardent/config.yaml`. For example:

```yaml
schema: 1
formats:
  markdown:
    files: ["*.md", "*.markdown"]
exclude: ["vendor/"]
compilation_cache: project
```

The project chooses which formats run. For each enabled format, project settings override the corresponding user keys. A
project list replaces the whole user list. Prefixes merge by prefix name. To inherit user excludes, omit `exclude` from
the project file. The `exclude: []` written by init explicitly clears the list.

Compilation uses a shared per-user cache by default. Set `compilation_cache: project` in either config for a separate
cache per project, or `compilation_cache: user` to share it. Both live under `$XDG_CACHE_HOME/ardent/compiled/` (default
`~/.cache/ardent/compiled/`). Clear `shared/` to discard the shared compiled code or `projects/<project-id>/` to discard
one project's compiled code. The project ID is the BLAKE3 digest of its canonical root path. If the cache is unavailable
or inside the current project, Ardent compiles in memory. Ardent recompiles on demand. Installed packages, settings and
structure files live outside these caches.

### Sync a project

For each tracked file, Ardent keeps:

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

### Browse a project

Run `ardent` in an interactive terminal anywhere inside the project to open the browser. You can also run
`ardent browse`. Run `ardent --help` to list the commands. The browser checks its input and output before syncing. The
explorer on the left groups tracked files by directory. Select a file and press Enter to open its outline on the right.
A horizontal divider marks the source pane below the outline. It shows the selected node's text. Repeated sibling labels
display as `item.1`, `item.2` or `Install.1`, `Install.2`. A lone node shows its original label. Press Tab to browse the
explorer while keeping the file open. In terminals narrower than 60 columns, Tab switches which pane fills the screen.

The outline colors labels by node kind and marks the selected row with `>`. The packaged Rust plugin highlights
keywords, strings and comments. The Markdown plugin highlights headings, inline code and code blocks. Files opened with
a scanner-only plugin have a plain source preview and the same navigation keys.

| Key | Action |
| --- | --- |
| Up / Down | Select the previous or next visible row. |
| Right | Expand a directory or outline node, or select its first child if expanded. |
| Left | Collapse a directory or outline node, or move to its parent. |
| Enter | In the explorer, toggle a directory or open a file. |
| Tab / Shift+Tab | Switch focus between the explorer and the open document. |
| [ / ] | Move to the previous or next sibling in the outline. |
| Page Up / Page Down | Scroll source text, project diagnostics, or the error for an open file. |
| Escape | Close the open document and return to the explorer. |
| q | Quit. |

When you save a tracked file in an editor, the browser runs sync in the background. You can navigate or quit while it
waits for the project lock. If the watcher cannot read a path, the file list shows a warning and other files stay
available. Changes to Git ignore rules also update the file list. If the project loses its `.ardent/` directory, the
browser exits and reports the missing project file.

After a refresh, the browser keeps the selected node if its node path still exists. It also keeps your place in the
source preview if the selected passage is unchanged. In the explorer, `!` marks a file with a scan error. Open it to
read the full message. The browser shows the diagnostic until the file parses again, then returns to the previous node
if it still exists.

### Supported outlines

The outline of a Markdown file holds headings, paragraphs, lists and their items, code blocks, block quotes, tables, and
front matter. A heading contains the blocks after it, up to the next heading of the same or a higher rank: `## Usage`
ends a `### Details` section. The text of a list item is part of the item, so its paragraphs are not separate nodes.

The outline of a Rust file holds its items and their members. Items are modules, impl blocks, traits, functions,
structs, enums, unions, constants, statics, type aliases, `macro_rules!` macros, and extern blocks. Members are fields,
enum variants, and the functions, constants, and types of traits and impl blocks. The label of an item is its keyword
and its name, such as `fn run` or `impl Display for Engine`. The doc comments and attributes above an item are part of
its node. `use` declarations, macro calls, and the statements in a function body are not nodes.

Format support (how to parse Rust, Markdown, and so on) and views (complexity, comments) come from **plugins**. A format
plugin receives source bytes and returns outline nodes or an error position. Query-based plugins package tree-sitter, a
grammar, and the outline query together. A view plugin stores or computes view entries.

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
- Decision 10 numbers generic labels in every node path, even a lone `paragraph.1`. Named siblings take positions only
  when their names repeat. A literal dot in a label is escaped in the path, so a heading named `Install.1` has the
  segment `Install\.1`.
- Decided in decision-2: every Ardent command first checks the tracked files for changes and re-scans the changed ones.
  The structure is a local cache; only stored views, the project configuration, and project plugins go into git.
- Decided in decision-4: the layout of `.ardent/` and the YAML schema of each file in it. Each YAML data file starts
  with a `schema` version. The specs `ardent-layout` and `ardent-schemas` in `openspec/specs/` give the details.
- Decision-3 defines view plugins. Ardent finds and rewrites references in their data. Decision-8 defines format plugins
  as WebAssembly components. A sync starts Wasmtime only to verify a changed or previously unverified plugin package or
  to scan a file.
- Decided in decision-5, during M1: an outline query can give a node a `level`, and the node then contains the siblings
  after it up to the next one with the same or a smaller level. Markdown headings nest this way.
- [Decision-9](backlog/decisions/decision-9%20-%20View-plugins-contribute-read-only-dashboards-that-Ardent-draws.md)
  defines read-only dashboards. A view plugin supplies an outline, board, or table for Ardent to draw. Recent activity
  uses the host-written `created` time on stored view entries.

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

Run `just` to list the development commands. `just fmt` formats Rust code. `just check` checks formatting, runs Clippy
and the workspace tests, builds the binary, and tests the install and format recipes. You can also run `just lint`,
`just test`, or `just build` separately.

Run `just install` to install the release binary and bundled format plugins. Cargo chooses the install root using its
normal configuration, including `CARGO_INSTALL_ROOT` and `install.root`. To override it, run
`just install /path/to/root`. Add the root’s `bin` directory to your `PATH`. Reinstalling replaces the bundled Markdown
and Rust packages.

To build and run directly with Cargo:

```sh
cargo build
cargo run -p ardent-cli   # the `ardent` binary
```

The host workspace has four crates. `ardent-core` holds the document model, structure files and configuration.
`ardent-format` verifies packages and runs components in Wasmtime. `ardent-cli` builds the `ardent` binary. `ardent-tui`
handles keyboard navigation and draws the browser. A separate guest workspace builds the two release packages. See the
[guest build instructions](guests/README.md).
