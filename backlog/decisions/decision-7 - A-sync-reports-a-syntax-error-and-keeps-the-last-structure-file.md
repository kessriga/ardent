---
id: decision-7
title: A sync reports a syntax error and keeps the last structure file
date: '2026-11-20 19:11'
status: accepted
---
## Context

tree-sitter recovers from syntax errors and always returns a tree. It marks a tree that contains errors
(`Node::has_error`). decision-3 left the use of this mark to a later task, and named the risk: without a rule, a person
who types in an editor while the TUI runs can turn links into orphans, and each status change rewrites view files.

TASK-8 requires that a file that does not parse gives a clear error for that file and does not stop the other files. In
the throwaway check of TASK-8, a half-typed `fn helper(` in an impl block also took in the next function, `fn after`.
The recovered outline lost both nodes.

In the design discussion of TASK-8, the user chose to report the error and keep the last structure file.

## Decision

- A parse tree that contains an `ERROR` or `MISSING` node has a syntax error. The outline engine then returns an error
  with the line and column of the first such node in document order, and no nodes.
- The sync reports the source path and the position as a file error, keeps the structure file of the last scan without a
  change, and continues with the other files. The command exits with status 1.
- The kept structure file has the stamp of the last content that parsed. The next sync finds a different hash and scans
  the file again. When the file parses, the sync writes a new structure file.
- A file that has no structure file gets none until it parses.

Alternatives:

- Write the outline that tree-sitter recovered, and print a warning. The outline stays current, but nodes in and after
  the broken region drop out, and their links become orphans while a person types. Rejected.
- Delete the structure file. The file then has no outline until it parses, and every link to it breaks. Rejected.

## Consequences

- Node paths stay stable while a file has a syntax error.
- A structure file can be older than its source file. The hash in its stamp then differs from the hash of the file, so a
  reader that needs current ranges can detect this.
- `ardent sync` exits with status 1 while a tracked file has a syntax error.
- The rule applies to every format plugin, not only to Rust.
- The open point of decision-3 about `Node::has_error` is closed.
