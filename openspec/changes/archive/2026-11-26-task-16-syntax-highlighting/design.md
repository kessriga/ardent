# Design

## Context

Decision-8 defines the required scanner and the host limits. The host already looks up the query-preparation export
separately. The browser loads a structure and checks its source hash before constructing a document. The terminal
renderer then slices the selected node's byte range and wraps graphemes by display width.

## Goals and scope

Compute styles once per loaded document. Selection, paging and redraw use those styles in memory. Sync, fingerprints,
node paths and the YAML schema keep their existing contracts.

## Proposed design

1. Extend `wit/format.wit` with an optional named highlighter export in a separate world. It accepts the same source
   bytes as the scanner and returns ordered, nonempty, nonoverlapping half-open byte ranges tagged with a syntax
   category. The initial categories are keyword, string, comment, heading and code. Keep the required scanner interface
   and its version intact.
2. The packaged guests use their grammars to identify tokens. Scope a Markdown heading to the heading syntax itself,
   since its outline node can include later paragraphs. Use the inline grammar for code spans and the block grammar for
   fenced and indented code. Inside a heading, give inline code the code style and split the surrounding heading range.
   Rust strings and comments own their full spans, including embedded keyword text.
3. Put the small, terminal-independent span and category types in `ardent-core`. In `ardent-format`, use the existing
   Wasmtime limits, optional export lookup and trap recovery. Validate the complete result before handing it to a
   caller. On an invalid range, a trap, or an unavailable export, the browser can still display the verified outline and
   plain source.
4. In `ardent-cli`, pass the verified formats from background sync to the browser. Match the format name and component
   digest to the structure stamp, then call the plugin on the bytes already checked by the browser. Store accepted spans
   with those bytes in the `ardent-tui` document. Refresh replaces the pair together. The highlight call runs only when
   loading a document.
5. In `ardent-tui`, intersect the sorted spans with the selected node's byte range. Carry original byte positions
   through the existing grapheme wrapping, tab and control display, and CRLF handling. Give each whole grapheme the
   style at its first byte. Keep the plain renderer's lossy display for invalid UTF-8. Apply kind colors to outline
   labels and preserve the bold row and selection marker independently.

## Risks and trade-offs

- Opening a document can require another parse. Keeping highlights with the loaded document avoids repeating it for
  every keypress and avoids adding a persistent cache.
- A plugin may return malformed offsets or too much data. Enforce the host transfer and execution limits, reject invalid
  bounds and ordering, and use plain text for that document.
- Unicode widths can hide a one-character alignment error in a text snapshot. Test terminal cell styles and symbols
  together before and after wrapping and paging.

## Compatibility

Existing installations can continue using their scanner-only packages. Rebuild the release and example Markdown and Rust
packages to supply highlighting. Verify plain previews with the existing scanner-only test fixture.
