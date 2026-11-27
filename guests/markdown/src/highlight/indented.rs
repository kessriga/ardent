//! The block grammar combines container markers and code indentation in a single continuation token.
use ardent_format_guest::highlighting::{Category, Span};
use tree_sitter::Node;

pub(super) fn append(spans: &mut Vec<Span>, block: Node<'_>, source: &[u8]) {
    let start = line_start(source, block.start_byte());
    let quotes = source[start..block.start_byte()]
        .iter()
        .filter(|&&b| b == b'>')
        .count();
    let indent = list_indent(block, source);
    let end = block.end_byte();
    // A block can end after the indentation of the following prose line.
    let end = if end < source.len() && !newline(source[end]) {
        line_start(source, end)
    } else {
        end
    };

    let mut offset = start;
    for line in source[start..end].split_inclusive(|&b| newline(b)) {
        let start = (offset + container_bytes(line, quotes, indent)) as u32;
        offset += line.len();
        let end = offset as u32;
        if start < end {
            if let Some(last) = spans.last_mut()
                && last.end == start
                && last.category == Category::Code
            {
                last.end = end;
            } else {
                spans.push(Span {
                    start,
                    end,
                    category: Category::Code,
                });
            }
        }
    }
}

fn container_bytes(line: &[u8], quotes: usize, indent: usize) -> usize {
    let after_quotes = line
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == b'>')
        .take(quotes)
        .last()
        .map_or(0, |(i, _)| i + 1);
    let mut column = width(&line[..after_quotes]);
    let target = quote_content_column(line, after_quotes, column) + indent;
    for (i, &byte) in line.iter().enumerate().skip(after_quotes) {
        if column >= target || newline(byte) {
            return i;
        }
        column = advance(column, byte);
    }
    line.len()
}

fn quote_content_column(line: &[u8], index: usize, column: usize) -> usize {
    column + usize::from(index > 0 && matches!(line.get(index), Some(b' ' | b'\t')))
}

// The innermost list marker supplies the content column relative to the last quote marker.
fn list_indent(mut node: Node<'_>, source: &[u8]) -> usize {
    while let Some(parent) = node.parent() {
        node = parent;
        if node.kind() == "block_quote" {
            break;
        }
        if node.kind() != "list_item" {
            continue;
        }
        let Some(marker) = node.named_child(0) else {
            break;
        };
        let prefix = &source[line_start(source, marker.start_byte())..marker.end_byte()];
        let Some(last) = prefix.iter().rposition(|b| !b.is_ascii_whitespace()) else {
            break;
        };
        let marker_end = width(&prefix[..=last]);
        let padding = width(prefix) - marker_end;
        let after_quote = prefix.iter().rposition(|&b| b == b'>').map_or(0, |i| i + 1);
        let quote_base = quote_content_column(prefix, after_quote, width(&prefix[..after_quote]));
        // Empty items and markers followed by indented code both use one column of padding.
        let has_content = source.get(marker.end_byte()).is_some_and(|&b| !newline(b));
        let padding = if has_content && (1..=4).contains(&padding) {
            padding
        } else {
            1
        };
        return marker_end + padding - quote_base;
    }
    0
}

fn line_start(source: &[u8], offset: usize) -> usize {
    let before = &source[..offset];
    before.iter().rposition(|&b| newline(b)).map_or_else(
        || {
            if before.starts_with(b"\xef\xbb\xbf") {
                3
            } else {
                0
            }
        },
        |i| i + 1,
    )
}

fn newline(byte: u8) -> bool {
    matches!(byte, b'\r' | b'\n')
}

fn advance(column: usize, byte: u8) -> usize {
    column + if byte == b'\t' { 4 - column % 4 } else { 1 }
}

fn width(bytes: &[u8]) -> usize {
    bytes.iter().fold(0, |column, &byte| advance(column, byte))
}
