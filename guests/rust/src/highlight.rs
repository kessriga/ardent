use ardent_format_guest::highlighting::{self, Category, Span};
use tree_sitter::{Node, Parser};

pub(super) fn spans(source: &[u8]) -> Vec<Span> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    highlighting::visit(&tree, |node| {
        let Some(category) = category(node, source) else {
            return true;
        };
        if !node.byte_range().is_empty() {
            spans.push(Span {
                start: node.start_byte() as u32,
                end: node.end_byte() as u32,
                category,
            });
        }
        false
    });
    spans
}

fn category(node: Node<'_>, source: &[u8]) -> Option<Category> {
    match node.kind() {
        "string_literal" | "raw_string_literal" => Some(Category::StringLiteral),
        "line_comment" | "block_comment" => Some(Category::Comment),
        "as" | "async" | "await" | "break" | "const" | "continue" | "crate" | "default" | "dyn"
        | "else" | "enum" | "extern" | "fn" | "for" | "gen" | "if" | "impl" | "in" | "let"
        | "loop" | "macro_rules!" | "match" | "mod" | "move" | "mutable_specifier" | "pub"
        | "raw" | "ref" | "return" | "self" | "static" | "struct" | "super" | "trait" | "try"
        | "type" | "union" | "unsafe" | "use" | "where" | "while" | "yield" | "boolean_literal" => {
            Some(Category::Keyword)
        }
        "type_identifier" | "identifier" if source.get(node.byte_range()) == Some(b"Self") => {
            Some(Category::Keyword)
        }
        _ => None,
    }
}
