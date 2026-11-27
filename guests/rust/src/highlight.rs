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
        "type_identifier" | "identifier" if source.get(node.byte_range()) == Some(b"Self") => {
            Some(Category::Keyword)
        }
        "identifier" | "default" | "union" if in_macro_tokens(node) => {
            let word = node.utf8_text(source).ok()?;
            (strict_keyword(word) || contextual_macro_keyword(node, word, source))
                .then_some(Category::Keyword)
        }
        "default" | "gen" | "macro_rules!" | "mutable_specifier" | "raw" | "try" | "union"
        | "yield" | "boolean_literal" => Some(Category::Keyword),
        kind if strict_keyword(kind) => Some(Category::Keyword),
        _ => None,
    }
}

fn in_macro_tokens(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "token_tree" | "token_tree_pattern" | "token_repetition" | "token_repetition_pattern"
        )
    })
}

// Macro arguments expose tokens rather than Rust expressions or item nodes.
fn contextual_macro_keyword(node: Node<'_>, word: &str, source: &[u8]) -> bool {
    // Macro token trees split a lifetime into an apostrophe and its name.
    if node.prev_sibling().is_some_and(|token| token.kind() == "'") {
        return false;
    }
    let next = || {
        std::iter::successors(node.next_sibling(), Node::next_sibling)
            .find(|token| !token.is_extra())
    };
    match word {
        "union" => next().is_some_and(|name| {
            matches!(
                name.kind(),
                "identifier" | "primitive_type" | "metavariable" | "union" | "default"
            ) && name
                .utf8_text(source)
                .is_ok_and(|word| !strict_keyword(word))
        }),
        "default" => matches!(
            next().and_then(|token| token.utf8_text(source).ok()),
            Some("impl" | "fn" | "type" | "const" | "unsafe" | "async" | "extern")
        ),
        "raw"
            if matches!(
                next().map(|token| token.kind()),
                Some("const" | "mutable_specifier")
            ) =>
        {
            std::iter::successors(node.prev_sibling(), Node::prev_sibling)
                .find(|token| !token.is_extra())
                .is_some_and(|token| matches!(token.kind(), "&" | "&&"))
        }
        _ => false,
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
