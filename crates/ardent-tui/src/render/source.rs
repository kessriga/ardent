//! Style the selected passage using offsets in the original source bytes.

use std::borrow::Cow;

use ardent_core::highlight::{Category, Highlight};
use ardent_core::theme::Theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;

use super::{Paging, page, styled_rows};
use crate::Page;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, open: &Page, theme: &Theme) -> Paging {
    let node = &open.document.outline.nodes[open.tree.selected];
    let Some(bytes) = open.document.bytes.get(node.range.clone()) else {
        return page(
            frame,
            area,
            std::iter::once(Line::from("Invalid source range")),
        );
    };
    let text = String::from_utf8_lossy(bytes);
    let spans = &open.document.highlights;
    let first = spans.partition_point(|span| span.range.end <= node.range.start);
    let lossy_bytes = if matches!(text, Cow::Owned(_)) {
        bytes
    } else {
        &[]
    };
    let style_at = source_style(lossy_bytes, node.range.start, &spans[first..], theme);
    page(
        frame,
        area,
        styled_rows(&text, area.width.into(), style_at).skip(open.scroll),
    )
}

fn source_style<'a>(
    lossy_bytes: &'a [u8],
    start: usize,
    mut spans: &'a [Highlight],
    theme: &'a Theme,
) -> impl FnMut(usize) -> Style + 'a {
    // Each invalid UTF-8 sequence becomes a three-byte replacement character in the displayed string.
    let mut replacements = lossy_bytes
        .utf8_chunks()
        .scan(0, |decoded, chunk| {
            if chunk.invalid().is_empty() {
                return None;
            }
            *decoded += chunk.valid().len() + '�'.len_utf8();
            Some((*decoded, '�'.len_utf8() - chunk.invalid().len()))
        })
        .peekable();
    let mut extra = 0;
    move |decoded| {
        while let Some((_, growth)) = replacements.next_if(|(end, _)| *end <= decoded) {
            extra += growth;
        }
        let offset = start + (decoded - extra);
        while let Some((first, rest)) = spans.split_first() {
            if first.range.end > offset {
                break;
            }
            spans = rest;
        }
        let Some(span) = spans.first().filter(|span| span.range.contains(&offset)) else {
            return Style::default();
        };
        if let Some(rgb) = theme.source.get(&span.category) {
            return Style::default().fg(super::rgb(rgb));
        }
        Style::default().fg(match span.category {
            Category::Keyword => Color::Magenta,
            Category::StringLiteral => Color::Green,
            Category::Comment => Color::Cyan,
            Category::Heading => Color::Yellow,
            Category::Code => Color::Green,
        })
    }
}
