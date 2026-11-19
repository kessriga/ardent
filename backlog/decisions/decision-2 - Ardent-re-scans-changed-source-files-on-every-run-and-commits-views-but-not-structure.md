---
id: decision-2
title: >-
  Ardent re-scans changed source files on every run and commits views but not
  structure
date: '2026-11-19 21:40'
status: accepted
---
## Context

People, agents, git, and formatters change source files while Ardent is not running. The structure in `.ardent/` then no
longer matches the source file, and view links can point at nodes that moved. decision-1 defines how links re-resolve
after a re-scan. It leaves three questions to this record: when re-scans run, which matcher maps the previous outline to
the new one, and where the previous outline is kept. This record also settles which parts of `.ardent/` go into git.

Three choices came from the design discussion:

- Ardent checks for changes at the start of every run. It needs no daemon and no git hooks.
- Git holds the views and the project configuration. The structure, the backlink index, and the file stamps stay local.
- Every sync writes its results, including rewritten links in view files, even when the command only reads.

The prior-art survey (doc-1) supports a rebuildable structure: Org-mode keeps its ID locations in a cache that a scan
can rebuild. No surveyed tool commits derived structure next to the source.

## Decision

The source file is authoritative. The structure is a local cache that Ardent rebuilds from the source file. The views
and the project configuration are user data and go into git.

### Authority

| Data                | Owner  | Rule during a sync                                                                            |
| ------------------- | ------ | --------------------------------------------------------------------------------------------- |
| Source file         | User   | The source file wins. Ardent never changes a source file to match the structure.              |
| Structure           | Ardent | The structure is stale. Ardent re-scans the source file and replaces the structure.           |
| View entry          | User   | Ardent keeps the entry. It rewrites only link paths, fingerprints, and statuses (decision-1). |
| Backlink index      | Ardent | Ardent rebuilds the index from the views.                                                     |
| Project config      | User   | Ardent reads it and does not change it during a sync.                                         |

Ardent never deletes a view entry during a sync. An entry whose anchor fails becomes an orphan, as decision-1 states.

### Sync

A *sync* brings the structure, the links, and the backlink index up to date with the source files. Every Ardent command
starts with a sync, and so does the TUI.

The structure of each source file records a *stamp*: the file's size, its modification time, a hash of its content, and
the name and version of the format plugin that built the outline.

1. Ardent lists the tracked files from the project configuration.
2. For each tracked file that has a structure, Ardent compares the format plugin name and version with the stamp. If
   either differs, the file changed.
3. Ardent compares the file's size and modification time with the stamp. If both are equal, the structure is current.
4. If either differs, Ardent hashes the content. If the hash equals the stamp, Ardent updates the size and time in the
   stamp, and the structure is current. If the hash differs, the file changed.
5. Ardent re-scans each changed file and each tracked file that has no structure. It matches the previous outline
   against the new outline (see "Matcher"). A tracked file that no longer exists loses its structure.
6. After all re-scans finish, Ardent re-resolves the links into changed and removed files, and retries orphans, by the
   rules of decision-1. A fingerprint search therefore sees every current outline, and a node that moved between two
   changed files resolves as `confirmed`.
7. Ardent writes the results (see "Writing results").

Ardent stamps view files in the same way. When a view file changed outside Ardent, for example after a git merge or an
edit by an agent, Ardent re-resolves the links in that file, adds missing `refs` items (decision-1), and rebuilds the
backlink index.

A sync reads the metadata of every tracked file. It hashes only the files whose size or time changed, and it matches
only the outlines of changed files.

While the TUI runs, a file watcher reports changes, and the TUI syncs the reported files after a short delay. The
watcher only makes updates faster. Correctness never depends on it: if the watcher misses an event, for example on a
network file system, the next sync finds the change through the stamps.

Ardent does not install git hooks and does not run a daemon. Git does not copy hooks when it clones a repository, and
hooks do not see saves from an editor. A daemon would be absent in most of the cases that this rule exists for.

### Writing results

A sync writes these files together:

- the new structure and stamp of each changed file;
- each view file in which a link changed: its path, fingerprint, status, or `was`;
- the backlink index.

The structure and the links must change together. If Ardent stored the new outline without the rewritten links, the next
sync would compare against the new outline and lose the rename.

Commands that only read, such as printing an outline, also sync and write. After an outside rename, a read command can
therefore leave changed view files in the working tree, as a formatter does.

Ardent writes each file to a temporary file and renames it into place. A lock file in the derived part of `.ardent/`
makes concurrent Ardent processes, such as a TUI and a CLI call from an agent, sync one at a time.

### Matcher

The previous outline is the structure from the last sync on this machine. Ardent maps its nodes to the new outline in
two phases, as GumTree does (doc-1), but on outline nodes instead of syntax trees:

1. **Exact phase.** Map nodes with the same node path. Then map nodes whose fingerprint occurs exactly once in each
   outline.
2. **Similarity phase.** For each unmapped old node, compare the unmapped new nodes of the same kind. For a node with
   children, the similarity is the share of its descendants that map into the candidate (GumTree's Dice coefficient).
   For a leaf, it is the similarity of the two texts. Prefer candidates whose parents map to each other. Map the best
   candidate above a threshold.

Each node maps to at most one node, as decision-1 requires. The implementing task picks the thresholds.

The table "Edits, walked through" in decision-1 is the acceptance test suite for the matcher. The implementing task
evaluates the `cinereus` crate, a maintained Rust implementation of GumTree-style matching, against that suite. If the
crate does not fit outline nodes, Ardent implements the two phases itself.

### What goes into git

`.ardent/` has two parts:

| Part      | Contents                                         | In git |
| --------- | ------------------------------------------------ | ------ |
| User data | View files, project configuration                | Yes    |
| Derived   | Structure files, stamps, backlink index, lock    | No     |

TASK-5 names the directories. It must keep the two parts in separate directories, so that one ignore rule covers the
derived part. `ardent init` writes `.ardent/.gitignore` with that rule. Ardent can delete the derived part at any time
and rebuild it with a full sync.

Git merges view files as text. When two branches change the same view entry, git reports a conflict, and the user
resolves it as in any other file. After the merge, a path that no longer resolves goes through the fingerprint search of
decision-1. To keep merges small, TASK-5 must:

- keep view entries in a stable order: append new entries, and do not re-sort when a path changes;
- keep data that Ardent derives across entries, such as counts, out of view files.

A fresh clone has no derived part. The first sync builds every structure without a previous outline. Links resolve by
path, then by fingerprint.

### What a later edit through the outline needs

Editing source files through the outline is a later milestone. This model leaves room for it. The feature needs:

1. **Byte ranges in the structure.** The structure keeps the byte range of each node, which the format plugin supplies
   (decision-1). The ranges are valid only while the hash of the source file equals the stamp.
2. **A checked write.** Before Ardent writes a source file, it hashes the file again. If the hash differs from the
   stamp, Ardent does not write. It syncs, finds the target node again, and asks the user to repeat the edit. Ardent
   writes to a temporary file and renames it into place.
3. **Splices.** Ardent replaces only the byte ranges of the edited nodes. The rest of the file keeps its formatting.
4. **Known mappings.** An edit through Ardent knows how its old nodes map to its new nodes. The sync after the edit uses
   this mapping instead of the matcher, so a rename through the outline gives `confirmed` links even when the body also
   changed.
5. **No double work.** After an edit, Ardent writes the new structure and stamp at once. When the watcher then reports
   the change, the stamp is already current.
6. **A source-edit operation in format plugins.** A plugin must turn an outline edit into a source edit: rename a node,
   move a subtree, insert a node. A rename can use the label byte ranges that plugins already supply. TASK-3 must leave
   room for this operation in the format plugin interface.

## Consequences

- Ardent works with any editor, with git, and with formatters. Nothing needs to run between Ardent commands.
- Each command reads the metadata of every tracked file. A very large project may need a faster check later, for example
  one that reads git's index. The stamp check stays the fallback.
- A read command can change view files after an outside rename or move. Agents and CI jobs that expect a clean working
  tree after a read must allow for this.
- Pull requests do not show the structure. A reviewer sees outline changes only through source diffs and through link
  rewrites in view files.
- A fresh clone has no previous outline. If someone renamed a node and edited its body without running Ardent, a fresh
  clone orphans the links to that node. A machine that synced before the change maps them as `unconfirmed`. A later
  improvement could rebuild the previous outline from git history.
- An upgrade of a format plugin re-scans every file of that format. If the upgrade changes labels, the matcher treats
  the changes as renames and rewrites the links.
- TASK-5 must split `.ardent/` into user data and a derived part, put the stamp in the structure file, name the lock
  file, and keep view entries in a stable order.
- TASK-3 must give each format plugin a name and a version, and leave room for the source-edit operation.
