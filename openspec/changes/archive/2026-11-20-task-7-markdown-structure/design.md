# Design

## Context

`ardent-core` has the module `layout`, with the `.ardent/` paths and `write_atomic`. `ardent-cli` has `ardent init` and
`project_root::find`. Nothing reads `config.yaml` or a source file yet.

The accepted decisions fix most of the shape:

- decision-1: node paths, labels, `[n]` suffixes, and fingerprints. Ardent core builds them from the node data that a
  format plugin supplies.
- decision-2: the sync and the stamp check.
- decision-3: the outline query (`@node`, `@label`, `kind`), the crate `ardent-format`, and built-in grammars that link
  natively.
- decision-4 and the specs `ardent-layout` and `ardent-schemas`: the paths and the YAML of structure files and
  `config.yaml`.
- decision-5 (new in this change): the query property `level`.

A throwaway check in the session scratchpad confirmed the base: `tree-sitter` 0.27.2 with `tree-sitter-md` 0.5.4 gives
the ranges of the spec example, and the fingerprint rule of `ardent-schemas` gives every fingerprint of that example.

## Goals / Non-Goals

**Goals:**

- One outline engine that is generic over format plugins. All Markdown knowledge stays in the query file.
- Structure files that match the spec examples byte for byte, apart from `mtime` and `format_hash`.
- A sync that writes nothing when no tracked file changed.

**Non-Goals:**

- A library API for other crates to run a sync. The sync lives in `ardent-cli`. TASK-9 moves it into a library crate if
  the TUI needs it there.
- Parallel scans. A sync scans files one at a time.

## Decisions

### Crates and modules

| Crate           | Module           | Holds                                                                                |
| --------------- | ---------------- | ------------------------------------------------------------------------------------ |
| `ardent-core`   | `outline`        | Node data from a format plugin, and the outline with paths and fingerprints.         |
| `ardent-core`   | `structure`      | The structure file: stamp, nodes, YAML read and write.                               |
| `ardent-core`   | `config`         | `config.yaml`: formats, views, excludes, name checks.                                |
| `ardent-core`   | `yaml`           | The serializer settings and the `schema` check that every Ardent YAML file shares.   |
| `ardent-core`   | `layout`         | Existing. Gains the structure file path and checks on relative paths.                |
| `ardent-format` | `engine`         | The format plugin type and the outline engine that runs its query.                   |
| `ardent-format` | `markdown`       | The built-in Markdown plugin and its `outline.scm`.                                  |
| `ardent-cli`    | `tracked`        | The walk that lists the tracked files and the warnings about the files it skips.     |
| `ardent-cli`    | `sync`           | The `ardent sync` command.                                                           |

`ardent-core` gets `serde`, `serde-saphyr`, `blake3`, and `jiff`. It gets no parser, terminal, or plugin-runtime
dependency. `ardent-format` gets `tree-sitter` and `tree-sitter-md`. `ardent-cli` gets `ignore`. The direction is
`ardent-core ← ardent-format ← ardent-cli`, as decision-3 states.

The node data from a format plugin is a tree of nodes, and each node has a kind, a label, a byte range, label ranges,
and children: exactly the list in decision-1. `ardent-core` turns this tree into the outline: the root node, the node
paths with escapes and suffixes, and the fingerprints, in document order.

### Outline engine

`ardent-format` runs the query of a format plugin over the parse tree in five steps:

1. Collect one candidate node for each match: the range of `@node`, the joined `@label` text and its ranges, `kind`, and
   `level`. Keep the first match for each syntax node.
2. Sort the candidates by start ascending and end descending, and nest them by containment with a stack.
3. In each list of siblings, apply the rule "Nesting by level" of the spec `outline` with a second stack. A node with a
   level moves the siblings after it under itself. Its range ends where the next sibling with the same or a smaller
   level starts, or at the end of the parent.
4. Normalize each label (spec `outline`, "Node label").
5. Return the tree to `ardent-core`.

The engine reads `kind` and `level` from the `#set!` properties of each pattern, and rejects a `level` that is not a
positive integer when it compiles the query.

### Markdown query

The query captures the heading lines (`atx_heading`, `setext_heading`), not the `section` nodes of tree-sitter-md. No
captured node then contains the blocks of a section, so all headings and top-level blocks of a file become siblings
under the root node. The `level` of each heading then nests them. Eight patterns set the levels: six for the ATX markers
`#` to `######`, and two for the setext underlines. The heading node starts at the heading line and ends at the next
heading of the same or a smaller level, so `Sync rules` in the spec example gets the range `50..172`, the range of its
tree-sitter section.

Alternative: capture `section` for ATX headings and use `level` only for setext headings. tree-sitter-md puts a setext
heading inside the section of the ATX heading above it, and a node cannot move out of a node that contains it. A level-1
setext heading under `## Old` would then stay inside `Old`. Rejected.

The other patterns:

- `paragraph`: only `(section (paragraph) @node)`, so a paragraph inside a list item, a quote, or a setext heading is
  not a node.
- `list`, `item` (`list_item`), `code` (`fenced_code_block` and `indented_code_block`), `quote` (`block_quote`), `table`
  (`pipe_table`), and `front_matter` (`minus_metadata` and `plus_metadata`): at every depth.

The kinds of decision-3's example query stay valid, so the spec example of `ardent-schemas` gives the same nodes.

### Built-in plugin metadata

The Markdown plugin is a Rust constant with its name, version, default patterns, and query text. The query is a real
file, `crates/ardent-format/src/markdown/outline.scm`, which the crate embeds with `include_str!`. A `.wasm` package of
the plugin can ship the same file later. The crate does not parse a `plugin.yaml` for built-in plugins; manifests come
with project plugins and the plugin host.

`format_hash` is the BLAKE3 hash of the query text, then the grammar crate name `tree-sitter-md`, then its version
`0.5.4`, with no separators. `Cargo.toml` pins `tree-sitter-md = "=0.5.4"`, so the version constant cannot drift from
the linked grammar.

### YAML

`serde-saphyr` 2.2 reads and writes every file, with these serializer settings: `compact_list_indent: false`,
`yaml_12: true`, `no_lang_directive: true`, `prefer_block_scalars: false`. With them, the output equals the structure
examples of `ardent-schemas` byte for byte. The serializer quotes a value only when YAML needs it, for example
`"docs/a.md#Note: this"`.

To read a file, Ardent first reads only the `schema` key, then reads the whole file into a type with
`#[serde(deny_unknown_fields)]`. So a file from a newer schema gets the message "unknown schema", not a message about a
key. Ranges (`start..end`) and `mtime` use serde `with` modules.

Alternatives: `serde_yaml` is unmaintained. `serde_yaml_ng` had its last release in 2024. A hand-written emitter would
have to get the YAML quoting rules right for labels such as `Note: this`.

### Stamp

`mtime` is the modification time as RFC 3339 UTC with nine fraction digits, through `jiff` (`{:.9}`), as in the spec
examples. `humantime` would panic on a time before 1970, which a file can have. The sync reads the metadata of a file
before its content. When the file changes between the two reads, the stamp has the old time and the new hash, so the
next sync hashes the file again and only updates the time.

### Tracked files

`ignore::WalkBuilder` walks from the project root with these settings:

- hidden files: included, because `.github/` and other dot-directories hold Markdown;
- `.ignore` files: not read, because decision-4 names only git's rules;
- `.gitignore` and `.git/info/exclude`: read, also when the project is not a git repository yet, so that `target/` stays
  untracked before `git init`;
- `.gitignore` files above the project root: read only inside a git repository, up to its root, as git does. Inside a
  repository, `require_git` is on, because only then does the `ignore` crate stop at the repository root. Otherwise it
  reads every `.gitignore` up to `/`, such as a `~/.gitignore` that git uses as its global excludes file. With
  `require_git` on, a nested git repository also gets no rule from outside it, as in git;
- the global git excludes file: not read, because it differs between machines, and node paths must not;
- symbolic links: not followed. A link to a file is not tracked, because git stores the link and not the content.

A filter skips `.git`, `.ardent`, and each path that an `exclude` pattern matches, so the walk never reads an excluded
directory. It stops at each directory that contains `.ardent/`, which it reports as a nested project. Format and
`exclude` patterns use `ignore::gitignore::GitignoreBuilder`, and the sync compiles them before it takes the lock, so an
invalid pattern writes nothing. An `exclude` list matches a path as git would ignore it, so `vendor/` excludes every
file below `vendor/`. A `files` list matches only the path of the file itself, so the files in a directory named
`notes.md/` do not become Markdown. The walk returns the tracked files sorted by path.

### Sync steps

1. Find the project root and read `config.yaml`. A configuration error ends the command before it writes anything.
2. Pick a plugin for each enabled format, warn about each format without a plugin, and compile the patterns.
3. Create `.ardent/cache/` if needed, and take the lock on `cache/lock` with `File::lock`. Create `cache/files/`.
4. Walk the tracked files.
5. If the walk reported no error, delete stale structure files and remove empty directories under `cache/files/`. This
   step comes before the scans. Where file names ignore letter case, `Readme.md` would otherwise reuse the directory of
   the old `README.md`, and this step would then delete it.
6. For each file, compare the stamp, then scan and write as the spec `sync` states. A file whose mtime is not earlier
   than the mtime of its structure file gets hashed even when the stamp matches, as git does for racily clean index
   entries. Collect file errors.
7. Print the summary to standard output, and the warnings and errors to standard error. Exit with status 1 if there was
   an error.

### Atomic writes

`write_atomic` checks `relative_path` first: it must be relative, and each component must be a normal name, so `..` and
absolute paths fail with `InvalidInput`. `ardent_dir` must already be a directory, so a wrong root fails with `NotFound`
and creates nothing. `write_atomic` then creates the parent directories of the target, as "Directories on demand"
requires. It creates each directory with `fs::create_dir` and checks it with `fs::symlink_metadata`. A symbolic link
committed into `.ardent/` therefore fails with `NotADirectory` and cannot send a write or a deletion outside the
project. `layout::create_dir` gives the lock and the pruning the same check.

### Project root

`project_root::find` returns `Result<Option<PathBuf>, (PathBuf, io::Error)>`. It checks each directory with
`fs::metadata`, continues on `NotFound` and `NotADirectory`, and returns every other error with the path it could not
check, the error shape that `ardent init` already uses. `ardent init` reports such an error and changes nothing.

The search starts at `env::current_dir()`, the path with symbolic links resolved, as git and cargo do. Alternative: the
logical path in `$PWD`. The walk does not follow symbolic links, so the files under a linked directory are not part of
the project anyway, and the logical path would find a root that does not track them. Rejected.

### Tests

- `ardent-core`: unit tests for escapes, suffixes, the fingerprint scenarios of `ardent-schemas`, the round trip of a
  structure file (write, read, compare), and `config.yaml` errors.
- `ardent-format`: the spec example of `ardent-schemas`, compared to the spec text. `insta` snapshots for files with
  nested headings, setext headings, lists, code blocks, front matter, quotes, and tables. `insta` stores each snapshot
  as a file next to the test, and `cargo insta review` shows changes as a diff.
- `ardent-cli`: integration tests that run `ardent sync` in temporary projects, one for each scenario of the specs
  `sync` and `ardent-layout` that a test can set up.

## Risks / Trade-offs

- [A file that changes twice within one tick of the file system clock, to the same size, keeps its old stamp] → git has
  the same limit. File systems with nanosecond times make it rare. The next change to the file fixes the structure.
- [Each sync reads the metadata of every tracked file and of every directory] → decision-2 accepts this and names a
  faster check for later.
- [Positional paths such as `list/item[2]` change when a person inserts an item above] → decision-1 names this weak
  spot. Re-scans keep stored paths current once the matcher exists.
- [A label from a setext heading inside a block quote keeps the `>` markers of its second line] → Rare. A later query
  version can fix it, and the new `format_hash` re-scans the files.
- [The exact pin of `tree-sitter-md` blocks automatic grammar updates] → An update is a deliberate change that also
  changes `format_hash` and re-scans every Markdown file. The pin makes this visible.
