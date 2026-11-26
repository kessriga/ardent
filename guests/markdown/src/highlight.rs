use ardent_format_guest::highlighting::{self, Category, Span};
use tree_sitter_md::{MarkdownParser, MarkdownTree};

mod indented;

pub(super) fn spans(source: &[u8]) -> Vec<Span> {
    let Some(tree) = MarkdownParser::default().parse(source, None) else {
        return Vec::new();
    };
    let mut cursor = tree.walk();
    let mut spans = Vec::new();
    loop {
        let node = cursor.node();
        let start = node.start_byte() as u32;
        let end = node.end_byte() as u32;
        let descend = match node.kind() {
            "atx_heading" | "setext_heading" => {
                spans.push(Span {
                    start,
                    end,
                    category: Category::Heading,
                });
                true
            }
            "indented_code_block" => {
                indented::append(&mut spans, node, source);
                false
            }
            "code_span" | "fenced_code_block" => {
                push_code(&mut spans, start, end);
                false
            }
            _ => true,
        };
        if descend && cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return exclude_continuations(&tree, spans);
            }
        }
    }
}

// The block grammar includes quote and list prefixes in multiline nodes, even after a closing fence.
fn exclude_continuations(tree: &MarkdownTree, spans: Vec<Span>) -> Vec<Span> {
    let mut prefixes = Vec::new();
    highlighting::visit(tree.block_tree(), |node| {
        if node.kind() == "indented_code_block" {
            return false;
        }
        if node.kind() == "block_continuation" && !node.byte_range().is_empty() {
            prefixes.push(node.start_byte() as u32..node.end_byte() as u32);
        }
        true
    });
    if prefixes.is_empty() {
        return spans;
    }
    let mut result = Vec::new();
    let mut prefixes = prefixes.iter().peekable();
    for mut span in spans {
        while let Some(prefix) = prefixes.peek() {
            if prefix.start >= span.end {
                break;
            }
            if span.start < prefix.start {
                result.push(Span {
                    end: prefix.start,
                    ..span
                });
            }
            span.start = span.start.max(prefix.end);
            if prefix.end > span.end {
                break;
            }
            prefixes.next();
        }
        if span.start < span.end {
            result.push(span);
        }
    }
    result
}

fn push_code(spans: &mut Vec<Span>, start: u32, end: u32) {
    // An inline code span can split the most recent heading into a prefix and a suffix.
    let heading = spans.pop_if(|span| span.category == Category::Heading && span.end > start);
    if let Some(ref heading) = heading
        && heading.start < start
    {
        spans.push(Span {
            start: heading.start,
            end: start,
            category: Category::Heading,
        });
    }
    spans.push(Span {
        start,
        end,
        category: Category::Code,
    });
    if let Some(heading) = heading
        && end < heading.end
    {
        spans.push(Span {
            start: end,
            end: heading.end,
            category: Category::Heading,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indented_code_keeps_its_whitespace_inside_containers() {
        for (source, expected) in [
            (
                "    one\r\n    two\r\n   prose",
                vec!["    one\r\n    two\r\n"],
            ),
            (
                "\u{feff}- A\n\n      one\n      two",
                vec!["    one\n", "    two"],
            ),
            ("\u{feff}    code", vec!["    code"]),
            ("\tone\n\ttwo", vec!["\tone\n\ttwo"]),
            (
                ">     one\n>     > code\n>   prose",
                vec!["    one\n", "    > code\n"],
            ),
            (
                "- Item\n\n      one\n\t\ttwo\n  prose",
                vec!["    one\n", "\ttwo\n"],
            ),
            (
                "-   \n      one\n      two\n  prose",
                vec!["    one\n", "    two\n"],
            ),
            ("> -     one\n >       two", vec!["    one\n", "    two"]),
        ] {
            let code: Vec<_> = spans(source.as_bytes())
                .into_iter()
                .filter(|span| span.category == Category::Code)
                .map(|span| &source[span.start as usize..span.end as usize])
                .collect();
            assert_eq!(code, expected, "{source:?}");
        }
    }

    #[test]
    fn quoted_headings_and_code_leave_container_markers_plain() {
        let source =
            "> # Title\n> Prefix `first\n> second` end.\n>\n> ```\n> body\n> ```\n> prose\n";
        let spans: Vec<_> = spans(source.as_bytes())
            .into_iter()
            .map(|span| {
                (
                    &source[span.start as usize..span.end as usize],
                    span.category,
                )
            })
            .collect();
        assert_eq!(
            spans,
            [
                ("# Title\n", Category::Heading),
                ("`first\n", Category::Code),
                ("second`", Category::Code),
                ("```\n", Category::Code),
                ("body\n", Category::Code),
                ("```\n", Category::Code),
            ]
        );
    }
}
