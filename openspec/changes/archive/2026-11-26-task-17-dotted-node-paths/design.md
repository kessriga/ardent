# Design

Decision 10 replaces the suffix rule in decision 1. In ardent-core, count original labels within each sibling group
before building paths. A label that comes from the kind, without a nonblank source label, is generic. Named and generic
nodes with the same spelling share a counter.

Core knows the sibling count when it builds the structure. Store the display label alongside the canonical path so
ardent-tui can draw it directly. The root displays the source path. Source file extensions keep their dots, and literal
dots in node labels use a backslash escape.

Write structure schema 2. Sync already rebuilds unsupported derived structures. The plugin interface and the other YAML
file schemas retain their existing versions.
