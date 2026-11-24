# Design

## Context

Decision-3 gives the host a fixed plugin clock and lets each view plugin define its fields. Decision-4 gives Ardent
ownership of stored entry IDs, anchors and references. A random ID lets concurrent additions land in different parts of
a file.

## Goals / Non-Goals

Give dashboards a common creation time for stored entries. A derived view entry can disappear and reappear on every
sync, so creation time applies only to stored view entries. The spike adds no crate, type or module.

## Decisions

The host records the time when it creates a stored entry or first reads one without a timestamp. A valid timestamp
travels with the entry through edits and moves. The dashboard sorts by the instant and breaks ties by ID. Files keep
their existing ID order.

A timestamp in a plugin field would require a shared convention across unrelated plugins. A file modification time would
make every comment look new whenever one changes. Adding the key to schema 1 now defines the format before the first
Ardent release that writes stored entries.

## Risks / Trade-offs

An entry written by an external tool without a timestamp first appears in recent activity at import time. An external
tool that knows the original creation time can supply it. Ardent reports an invalid timestamp so the author can repair
it without losing entries.
