---
id: decision-10
title: Use dotted positions for generic nodes and repeated sibling names
date: '2026-11-26 13:27'
status: accepted
---
## Context

Decision 1 introduced positional suffixes for repeated sibling labels. The browser also needs to distinguish the first
member of a repeated group and to display a short name for a singleton.

## Decision

This replaces the positional suffix rule in decision 1. Core groups children by their original label within each parent.
Generic labels supplied by the node kind always receive a one-based dotted position. Named nodes receive a position only
when their name repeats. A literal dot in a label is escaped in the path. The root keeps its file path.

## Consequences

A lone paragraph has the address segment `paragraph.1` and displays as `paragraph`. Two headings named Install have the
segments and display labels `Install.1` and `Install.2`. A lone heading named Install keeps the segment `Install`. Sync
rebuilds derived structures. Version 0.0.0 provides no migration of stored references to old addresses.
