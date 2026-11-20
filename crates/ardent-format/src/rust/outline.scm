; Outline query of the built-in Rust format plugin (spec rust-format).
;
; Each item, field, and enum variant is a node. The outer doc comments and attributes directly before it are
; annotations, so they belong to its node (decision-6). These patterns name no parent: in a pattern with a parent, a run
; of zero annotations before the anchor `.` would match only the first child of that parent.
;
; Labels are the keyword and the name. A negative impl leaves out its `!`, because the engine joins label captures with
; spaces.

; Items

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (mod_item "mod" @label name: (_) @label) @node
  (#set! kind "mod"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (impl_item "impl" @label trait: (_)? @label "for"? @label type: (_) @label) @node
  (#set! kind "impl"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (trait_item "trait" @label name: (_) @label) @node
  (#set! kind "trait"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (function_item "fn" @label name: (_) @label) @node
  (#set! kind "fn"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (function_signature_item "fn" @label name: (_) @label) @node
  (#set! kind "fn"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (struct_item "struct" @label name: (_) @label) @node
  (#set! kind "struct"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (enum_item "enum" @label name: (_) @label) @node
  (#set! kind "enum"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (union_item "union" @label name: (_) @label) @node
  (#set! kind "union"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (const_item "const" @label name: (_) @label) @node
  (#set! kind "const"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (static_item "static" @label name: (_) @label) @node
  (#set! kind "static"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (type_item "type" @label name: (_) @label) @node
  (#set! kind "type"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (associated_type "type" @label name: (_) @label) @node
  (#set! kind "type"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (macro_definition "macro_rules!" @label name: (_) @label) @node
  (#set! kind "macro"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (foreign_mod_item (extern_modifier) @label) @node
  (#set! kind "extern"))

; Fields and enum variants

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (field_declaration name: (_) @label) @node
  (#set! kind "field"))

(
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)]* @annotation
  .
  (enum_variant name: (_) @label) @node
  (#set! kind "variant"))

; A field of a tuple struct or a tuple variant is a bare type in its list, so these patterns name the parent. The first
; takes the fields with annotations, which here also include the visibility; the engine keeps the first pattern that
; matches a syntax node.

(ordered_field_declaration_list
  [(line_comment outer: (outer_doc_comment_marker))
   (block_comment outer: (outer_doc_comment_marker))
   (attribute_item)
   (visibility_modifier)]+ @annotation
  .
  type: (_) @node
  (#set! kind "field"))

(ordered_field_declaration_list
  type: (_) @node
  (#set! kind "field"))
