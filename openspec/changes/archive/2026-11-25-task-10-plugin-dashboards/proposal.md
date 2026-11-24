# Proposal

## Why

TASK-10 needs a creation time to list recent comments and recent activity across stored views. Random view entry IDs
identify entries and keep git diffs small. They carry no date.

## What Changes

- Add the host-written `created` timestamp to stored view entries in schema 1.
- Specify how Ardent handles external additions, edits and moves. Keep the existing rules for IDs and file order.
- Record how view plugins supply dashboards, how Ardent draws them, and why source dashboards are read-only.

## Capabilities

### Modified Capabilities

- `ardent-layout` specifies per-dashboard settings for the planned view host.
- `ardent-schemas` gains a creation time in its stored view entry requirements and worked examples.

## Impact

This spike defines the contract before Ardent ships a stored view writer. Implementing the view host, dashboard settings
parser, renderer and stored view writer falls outside TASK-10.
