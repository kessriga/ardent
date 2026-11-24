---
id: decision-9
title: View plugins contribute read-only dashboards that Ardent draws
date: '2026-11-24 21:45'
status: accepted
---
## Context

Decision-3 lets each view plugin define its fields and gives the host control of its clock. Decision-4 gives Ardent
ownership of IDs and anchors. A dashboard can group Backlog.md tasks by status only if it understands that plugin's
fields. Ardent can show all tracked files using its own data.

## Decision

**Discovery.** A stored or derived view component may also export the versioned interface
`ardent:plugin/dashboards@0.1.0`. After binding the required view interface, Ardent looks up this interface on the same
instance. Absence means the plugin contributes no dashboards. If present, both functions below must have the expected
types. Report a malformed dashboard export against its plugin. The base view still works if its own interface is valid.

| Call | Result |
| --- | --- |
| `list()` | An ordered list of descriptors. Each has a stable ID unique within the plugin and a display title. Ardent identifies a dashboard by plugin name and ID. |
| `render(id, now, settings, entries)` | One outline, board or table, or an error message. Ardent reports errors with the plugin name and dashboard ID. |

**Inputs and capabilities.** After sync, the host passes the selected dashboard ID, a single RFC 3339 UTC instant for
this render, the dashboard settings, and a project-wide snapshot of this plugin's view entries. Each stored row includes
its ID, creation time, anchor with resolution status, and plugin fields. Each derived row includes its node path and
plugin fields. The host passes its own time explicitly because the guest clock is fixed. A day-based dashboard can take
its time zone from settings and defaults to UTC. Dashboard calls have the read-only `outline-of(file)` and
`backlinks(target)` functions of decision-3, under the same runtime limits. The dashboard has no host function to write
source files or view entries.

**Settings.** Configure each dashboard under `views.<plugin>.dashboards.<id>` in `.ardent/config.yaml`. The plugin
receives the named fields for that ID, using the flat field value types of decision-3. Missing settings mean an empty
set and the plugin's defaults. The plugin validates the settings. For example, this limits the comments plugin's recent
list to 20 entries:

```yaml
views:
  comments:
    dashboards:
      recent:
        limit: 20
```

**Rendering.** Ardent owns screen layout, focus and navigation. Plugins return ordered data and plain text. Each
selectable item may carry a node path or stored entry ID for Ardent to open. An orphaned entry remains reachable by its
ID. Ardent validates targets and layout shape before drawing. Invalid data produces a diagnostic for that dashboard.

| Layout | Data in display order |
| --- | --- |
| Outline | A flat list of items with a label, optional target and optional index of an earlier parent. An absent parent makes the item a root. |
| Board | A list of named lanes. Each lane contains a list of cards with a label and optional target. Empty lanes are valid. |
| Table | A list of column titles and a list of rows. Each row has one text cell per column and an optional target. |

**Built-in dashboards.** The file explorer groups tracked source files by directory and opens a file's root node. Recent
activity lists additions to stored views across the project, newest creation time first, then ID in byte order. Ardent
can label a row with the view name and anchor path and open it by ID. Derived entries have no creation time.

**Backlog.md board.** The derived backlog plugin examines the source bytes it receives during compute. For a tracked
`backlog/tasks/*.md` file it attaches a derived entry to the file root with its task ID, title and status from front
matter. The board groups these entries into status lanes. For example, TASK-10 appears under In Progress, and selecting
its card opens the task file. To move the card, the user changes the task status in Backlog.md. After sync, the plugin
recomputes the entry and the board shows it in its new lane. The board is read-only.

**Recent comments.** Given two comments created at 08:10 and 08:16 UTC, the comments plugin returns an outline with the
08:16 comment first. It uses the text field for each label and the entry ID for its target. It sorts by creation instant
descending, then ID in byte order, and applies its limit. Editing the older comment leaves it in second place.

**Storage.** Ardent writes `created` on stored entries in schema 1 when it creates them. A sync fills a missing value on
an otherwise valid external entry with the time it first sees the entry. A valid supplied time survives edits, ID
repairs and moves. An invalid time produces a diagnostic and preserves the file. Derived entries keep their existing
shape. Files retain ID order. The `ardent-schemas` spec defines the exact spelling and examples.

## Consequences

A plugin can add dashboards without drawing a terminal or changing the base view interface. The snapshot cost grows with
the number of entries in that view. Source changes still pass through the source tool, which can enforce its own status
rules. An outside entry first seen without a timestamp appears in recent activity at import time.

The throwaway check at `spikes/task-10/check_dashboards.py` loads two minimal components into one Wasmtime host. Both
return base view metadata. Only the extended component exposes the dashboard interface and returns the Recent comments
descriptor. This checks discovery and invocation. Rendering and the complete view plugin ABI belong to the future view
host.
