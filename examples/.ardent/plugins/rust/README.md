# Rust format plugin

This component uses tree-sitter-rust 0.24.2. Its grammar rejects Rust 2024 items marked `safe` inside an `unsafe extern`
block, including the valid declaration `unsafe extern "C" { pub safe fn f(); }`.

When the grammar rejects a file, `ardent sync` reports its path, line and character column. Sync keeps its last good
structure, continues with other files and exits with status 1. A file that has never parsed successfully has no
structure.
