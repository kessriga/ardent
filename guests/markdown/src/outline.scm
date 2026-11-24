; Outline query of the Markdown format plugin (spec markdown-format).
;
; Headings are captured as lines, not as tree-sitter sections. Their level nests the blocks after them (decision-5),
; so ATX and setext headings nest the same way.

((atx_heading (atx_h1_marker) heading_content: (inline)? @label) @node
  (#set! kind "heading")
  (#set! level "1"))

((atx_heading (atx_h2_marker) heading_content: (inline)? @label) @node
  (#set! kind "heading")
  (#set! level "2"))

((atx_heading (atx_h3_marker) heading_content: (inline)? @label) @node
  (#set! kind "heading")
  (#set! level "3"))

((atx_heading (atx_h4_marker) heading_content: (inline)? @label) @node
  (#set! kind "heading")
  (#set! level "4"))

((atx_heading (atx_h5_marker) heading_content: (inline)? @label) @node
  (#set! kind "heading")
  (#set! level "5"))

((atx_heading (atx_h6_marker) heading_content: (inline)? @label) @node
  (#set! kind "heading")
  (#set! level "6"))

((setext_heading heading_content: (paragraph (inline) @label) (setext_h1_underline)) @node
  (#set! kind "heading")
  (#set! level "1"))

((setext_heading heading_content: (paragraph (inline) @label) (setext_h2_underline)) @node
  (#set! kind "heading")
  (#set! level "2"))

; Only a paragraph directly in the document body is a node. The text of a list item, a block quote, or a setext
; heading belongs to that node.
((section (paragraph) @node)
  (#set! kind "paragraph"))

((list) @node
  (#set! kind "list"))

((list_item) @node
  (#set! kind "item"))

((fenced_code_block) @node
  (#set! kind "code"))

((indented_code_block) @node
  (#set! kind "code"))

((block_quote) @node
  (#set! kind "quote"))

((pipe_table) @node
  (#set! kind "table"))

((minus_metadata) @node
  (#set! kind "front_matter"))

((plus_metadata) @node
  (#set! kind "front_matter"))
