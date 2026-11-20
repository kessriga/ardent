---
id: decision-5
title: Outline queries can nest nodes by a level property
date: '2026-11-20 13:14'
status: accepted
---
## Context

decision-3 makes a format plugin data, not code: a tree-sitter grammar and an outline query. The parent of a node is the
nearest `@node` whose range contains it. This rule works when the syntax tree already nests the nodes, as it does for
Rust items and for ATX headings in Markdown, which `tree-sitter-md` wraps in `section` nodes.

decision-3 also notes that `tree-sitter-md` makes no section for a setext heading, a title underlined with `===` or
`---`. The content below a setext heading therefore does not nest under it, and decision-3 left the fix to TASK-7. A
setext heading of level 1 can also sit inside the section of an ATX heading of level 2, so no capture of the syntax tree
gives the right parent.

Prose formats show this pattern often: a heading is a flat line, and its level, not the syntax tree, decides which
blocks belong to it. HTML (`<h1>` to `<h6>`) and AsciiDoc have the same shape.

In the design discussion of TASK-7, the user chose to nest setext headings in code rather than to leave them flat, and
then chose a generic query property over a Markdown-only hook.

## Decision

An outline query can set the property `level` on a pattern, for example `(#set! level "2")`. The value is a positive
integer. This adds one row to the query table of decision-3:

| Query part          | Meaning                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------ |
| `(#set! level "2")` | The node contains the siblings after it up to the next sibling with the same or a smaller level. |

The outline engine applies the rule after it nests the nodes by containment:

1. In each list of siblings, a node with a level takes the siblings after it as its children, up to the next sibling
   with the same or a smaller level. A sibling without a level never ends the run.
2. The range of the node ends where that next sibling starts, or at the end of the parent when no such sibling follows.
3. The engine rejects a query with a `level` that is not a positive integer, and builds no outline with it.

The built-in Markdown plugin captures the heading lines, not the `section` nodes, and sets the level of each ATX and
setext heading. All Markdown knowledge stays in its query. The range of an ATX heading node then equals the range of its
tree-sitter section, so the structure example of the spec `ardent-schemas` does not change.

## Consequences

- Setext headings nest like ATX headings, and a setext heading of level 1 can end the run of an ATX heading of level 2.
- A future HTML or AsciiDoc plugin can nest its headings with the same property and no code.
- The outline engine has one more rule, and the outline of a query depends on the order of siblings as well as on
  containment.
- A format that needs other processing still needs a new query property or the optional plugin component that decision-3
  names. This decision does not add a general hook for format code.
- The spec `outline` holds the normative rule ("Nesting by level"). The spec `markdown-format` holds the Markdown
  levels.
