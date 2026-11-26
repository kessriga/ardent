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
        "default" | "gen" | "macro_rules!" | "mutable_specifier" | "raw" | "try" | "union"
        | "yield" | "boolean_literal" => Some(Category::Keyword),
        "type_identifier" | "identifier" if source.get(node.byte_range()) == Some(b"Self") => {
            Some(Category::Keyword)
        }
        // Macro token trees expose some strict keywords, such as `else`, as identifiers.
        "identifier"
            if node.utf8_text(source).is_ok_and(strict_keyword)
                && node.parent().is_some_and(|parent| {
                    matches!(
                        parent.kind(),
                        "token_tree"
                            | "token_tree_pattern"
                            | "token_repetition"
                            | "token_repetition_pattern"
                    )
                }) =>
        {
            Some(Category::Keyword)
        }
        kind if strict_keyword(kind) => Some(Category::Keyword),
        _ => None,
    }
}

fn strict_keyword(word: &str) -> bool {
    matches!(
        word,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
}
