---
id: decision-6
title: Outline queries can widen a node over annotation captures
date: '2026-11-20 19:11'
status: accepted
---
## Context

decision-3 makes the byte range of the `@node` capture the range of a node. In `tree-sitter-rust`, an outer doc comment
(`///` or `/** */`) and an attribute (`#[derive(Debug)]`) are siblings before the item they apply to, not children of
it. A `@node` capture of the item therefore leaves them out.

TASK-8 requires that doc comments and attributes are part of the node they apply to. A move of the node must carry them,
and an edit of a doc comment must change the fingerprint of its item, not the fingerprint of the parent.

A query can match a run of siblings before an item, for example
`[(attribute_item) (line_comment outer: (outer_doc_comment_marker))]* @x . (function_item) @node`. A throwaway check
with `tree-sitter` 0.27.2 and `tree-sitter-rust` 0.24.2 gave one match for each item, with every doc comment and
attribute of the run before it. A blank line does not end the run. An ordinary `//` comment ends it.

Zed's outline queries use the capture name `@annotation` for such text. In the design discussion of TASK-8, the user
chose a separate capture over several `@node` captures in one match.

## Decision

An outline query can have `@annotation` captures. This adds one row to the query table of decision-3:

| Query part    | Meaning                                                                               |
| ------------- | ------------------------------------------------------------------------------------- |
| `@annotation` | Text that belongs to the node of the match, such as a doc comment. The range covers it. |

- The range of a node starts at the smallest start and ends at the largest end of the `@node` capture and the
  `@annotation` captures of its match.
- The `@node` capture still identifies the node. Ardent uses its syntax node for the rule "one node for each syntax
  node", and to order two nodes with the same range.
- The fingerprint uses the whole range, so it includes the annotations. The label ranges do not change.

The built-in Rust plugin captures the outer doc comments and outer attributes before each item, field, and enum variant
as annotations.

Alternative: allow several `@node` captures in one match, and let the range cover all of them. No capture then says
which syntax node identifies the node. Rejected.

## Consequences

- A doc comment or an attribute moves with its item. An edit to it changes the fingerprint of the item.
- An ordinary comment between a doc comment and its item ends the run, so the doc comment stays in the range of the
  parent. rustc attaches the doc comment to the item. A later query version can include such comments, and its new
  format hash re-scans the files.
- A query repeats the annotation part in each pattern that needs it.
- The spec `outline` holds the normative rule.
