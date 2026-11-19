---
id: decision-1
title: >-
  Anchors and references address nodes by readable path with a content
  fingerprint
date: '2026-11-19 21:21'
status: accepted
---
## Context

A view entry, such as a comment on a function, needs an anchor that attaches it to a node. View content also holds
references, such as `@foo`, that point at nodes anywhere in the project, including sections of a Markdown spec. Source
files change all the time, often outside Ardent, and Ardent must never write into them. The README promises one
addressing scheme for anchors and references.

The prior-art survey (doc-1) found no tool that keeps a reference stable across outside edits without writing an ID into
the source file or owning the text. Tools that may not touch the document (Hypothesis, GitHub, Gerrit) store redundant
selectors, re-anchor after edits, and keep failed anchors as orphans. This decision follows that model.

Two requirements came from the design discussion:

- A person writes a reference in Ardent with a picker. AI agents and the CLI write references without a picker, so the
  stored form must be a text address that resolves to one node.
- When Ardent is unsure where a node went, it rebinds the anchor or reference to its best candidate and marks it for the
  user to confirm. It does not leave it orphaned.

## Decision

The **node path** is the identity of a node. Anchors and references store a node path plus a **fingerprint** of the
node's content. There are no opaque node IDs.

### Node path

A node path is a project-relative file path, `#`, and the labels of the nodes from the file's root down to the target,
joined by `/`:

```text
docs/spec.md#Sync rules/Renames
src/engine.rs#impl Engine/fn run
src/engine.rs
```

- A path without `#` addresses the root node of the file.
- The format plugin gives each node its label. A label is one line, with runs of whitespace collapsed to one space.
- When a parent has more than one child with the same label, the first child in document order keeps the bare label. The
  next ones get the suffixes `[2]`, `[3]`, and so on: `Example`, `Example[2]`.
- A backslash escapes `/`, `#`, `[`, `]`, and `\` inside a label or a file path: `Read\/write`.
- A node without a natural name, such as a Markdown paragraph, uses its kind as its label. Its siblings then get
  positional suffixes (`paragraph[3]`). Re-scans keep anchors on such nodes correct, but a typed path to one breaks when
  siblings are inserted above it.

### Fingerprint

The fingerprint is a hash of the node's source text, with these changes before hashing:

1. Remove the byte ranges that the label comes from.
2. Collapse each run of whitespace to one space, and trim the ends.

The text covers the whole node, children included. A rename keeps the fingerprint. A body edit, or an edit to any child,
changes it. TASK-5 picks the hash function and its width; the hash must give the same result on every platform and in
every Ardent version.

### Anchors

An anchor stores the node path, the fingerprint, and a status. It can narrow to part of the node with a `quote` of the
selected text and a `range`, given as line offsets from the node's first line. The quote is the selector; the range only
picks between several copies of the quote.

```yaml
anchor:
  path: src/engine.rs#impl Engine/fn run
  fingerprint: 9f2c41d07be3a815
  status: confirmed
```

Anchors store no byte offsets. The field spellings in this record show the meaning; TASK-5 fixes the schema.

### References

A reference inside view content has the form `@[<node path>]`:

```text
This function contradicts @[docs/spec.md#Sync rules] and also function @[src/sync.rs#fn bar].
```

- The reference ends at the first unescaped `]`. An `@` that is not followed by `[` is plain text.
- References point at whole nodes. Only anchors can narrow to a range.
- In the TUI, a person types `@`, and a picker searches the outlines of every source file. A prefix such as `@spec:foo`
  narrows the search to the files that a format plugin assigns to the prefix `spec`. When the person picks a node,
  Ardent inserts `@[<node path>]`. The typed query is not stored.
- Agents and the CLI write `@[<node path>]` directly.
- The TUI shows a reference as `@` plus the last label of its path, for example `@Sync rules`. For a file root, it shows
  the file name.
- Each view entry has a `refs` list with one item for each distinct path in its content. Each item holds the path, the
  fingerprint, and a status. Ardent writes this list. When an agent writes a reference with no `refs` item, Ardent adds
  the item at the next scan, if the path resolves.

```yaml
- anchor:
    path: src/engine.rs#impl Engine/fn run
    fingerprint: 9f2c41d07be3a815
    status: confirmed
  text: This function contradicts @[docs/spec.md#Sync rules] and also function @[src/sync.rs#fn bar].
  refs:
    - path: docs/spec.md#Sync rules
      fingerprint: 03be77a2c4d1e9f0
      status: confirmed
    - path: src/sync.rs#fn bar
      fingerprint: 5d81c0aa9e2f4b63
      status: unconfirmed
      was: src/sync.rs#fn foo
```

### Statuses

In the rest of this record, a *link* is an anchor or a `refs` item. Each link has one of three statuses:

| Status        | Meaning                                                    | What the user sees                                               |
| ------------- | ---------------------------------------------------------- | ---------------------------------------------------------------- |
| `confirmed`   | The path addresses the right node.                         | The current name, such as `@baz`.                                |
| `unconfirmed` | Ardent rebound the link to its best candidate.             | `@baz`, marked "was @foo?". The user confirms or picks a node.   |
| `orphan`      | No node matches.                                           | `@foo`, dimmed, with its last known path. The user picks a node. |

An `unconfirmed` link keeps its previous path in `was`. When the user confirms it, Ardent sets `confirmed` and removes
`was`. When the user picks another node instead, Ardent rewrites the path, sets `confirmed`, and removes `was`. Ardent
never raises an `unconfirmed` link to `confirmed` by itself; only the user does.

### Re-resolution

When a path changes, Ardent rewrites it in the anchor, in the inline `@[…]` text, and in the `refs` item. Ardent
re-resolves links in two ways. Each rule below gives the highest status the link can get. A link that is already
`unconfirmed` stays `unconfirmed`.

**At a re-scan of a changed file.** Ardent matches the file's previous outline, stored in `.ardent/`, against the new
outline. TASK-4 decides when re-scans run and which tree-matching algorithm to use. Ardent finds the links into the file
through the backlink index. For each link:

1. The old node maps to a new node with the same path: `confirmed`. Refresh the fingerprint.
2. The old node maps to a new node with a different path, and the new node has the link's fingerprint: rewrite the path,
   `confirmed`.
3. The old node maps to a new node with a different path and a different fingerprint: rewrite the path, `unconfirmed`.
4. The old node maps to no node: use the fingerprint search below.

**For a path that is not in any current outline,** for example after a git merge, a typo by an agent, or a lost
structure cache, Ardent searches the current outlines of all tracked files for the link's fingerprint:

1. Exactly one node has the fingerprint: rewrite the path, `confirmed`.
2. More than one node has the fingerprint: pick the one in the same file, else the one with the longest common path
   prefix. Rewrite the path, `unconfirmed`.
3. No node has the fingerprint: `orphan`.

A path that is in a current outline addresses that node, even when the fingerprint differs. The path is the identity, so
Ardent refreshes the fingerprint and keeps the status.

Ardent retries orphans at every scan, so a link comes back when its node returns, for example after a revert.

For an anchor with a `quote`, Ardent first resolves the node. It then searches the node for the quote and uses the
`range` to choose between copies. If the quote is gone, the anchor falls back to the whole node as `unconfirmed`, and
keeps the quote so the user sees what it was attached to. Confirmation removes the quote and the range.

### Edits, walked through

| Edit                                    | Result                                                                                       |
| --------------------------------------- | -------------------------------------------------------------------------------------------- |
| Insert lines above the node             | Same path. `confirmed`. Nothing to rewrite, because anchors store no offsets.                |
| Edit the node body                      | Same path. `confirmed`. Fingerprint refreshed.                                               |
| Rename the node                         | The matcher maps old to new; same fingerprint. Path rewritten, `confirmed`.                  |
| Rename and rewrite the body in one edit | Partial match: path rewritten, `unconfirmed`. No mapping and no fingerprint match: `orphan`. |
| Move the node to another parent         | Same fingerprint. Path rewritten, `confirmed`.                                               |
| Move the node to another file           | Unmapped in the old file; fingerprint search finds it. Path rewritten, `confirmed`.          |
| Rename or move the file                 | Every path into the old file fails; fingerprint search finds each node.                      |
| Delete the node                         | No mapping, no fingerprint match: `orphan`.                                                  |
| Split one node into two                 | The matcher maps the old node to one part: `unconfirmed`.                                    |
| Merge two nodes into one                | The node the matcher maps becomes `unconfirmed`. The other old node's links become orphans.  |

### Orphaned view entries

Ardent never deletes a view entry because its anchor is an orphan. The entry shows at the top of its file's outline, in
a group of orphans. If the file is gone, the entry shows in a project-wide list of orphans. The user re-points the entry
with the picker or deletes it.

### What a format plugin supplies

For each node of an outline:

- its kind, such as `fn` or `heading`;
- its label;
- its byte range in the source file;
- the byte ranges that the label comes from;
- its children, in document order.

For the plugin as a whole: the search prefixes it offers, such as `spec`, and the files each prefix covers.

Ardent core builds node paths, escapes labels, adds `[n]` suffixes, computes fingerprints, matches outlines, keeps the
statuses and the `refs` lists, and maintains the backlink index. The backlink index maps each node path to the view
entries that reference it. Ardent derives it from the views and can rebuild it at any time.

## Consequences

- People and agents can read and write every anchor and reference in `.ardent/` without a lookup table.
- The structure stays a cache. Ardent can rebuild it from the source files without losing any link identity.
- A rename rewrites every view entry that points at the node. Diffs of `.ardent/` get noisier, and two branches can
  conflict on the same view file. A stale path that a merge brings back resolves through its fingerprint.
- Re-resolution after a rename needs the previous outline. TASK-4 must keep it in `.ardent/` and choose the matcher.
  Without it, path lookup and fingerprint search still work; only a rename combined with a body edit becomes an orphan.
- A node moved to another file leaves its view entry in the old file's view. TASK-5 decides whether Ardent moves the
  entry to the new file's view.
- TASK-5 must fix the YAML spellings: anchor fields, `refs`, the status values, `was`, and the fingerprint hash.
- TASK-3 must give format plugins a way to return the node data listed above, and to declare search prefixes.
- Positional `[n]` suffixes and unnamed nodes are the weak spot of typed paths. Re-scans keep stored paths current; a
  path typed by hand to such a node can address the wrong sibling after an edit.
- Merging two nodes orphans the links of one of them, because the matcher maps old nodes to new nodes one to one.
