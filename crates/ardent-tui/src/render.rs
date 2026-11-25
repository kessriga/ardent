//! Lay out the file list, outline, and source preview in terminal cells.

use std::borrow::Cow;

use unicode_width::UnicodeWidthChar;

use crate::{Browser, Page};

pub(super) struct Frame {
    pub rows: Vec<String>,
    pub preview_height: usize,
    pub more: bool,
}

impl Frame {
    pub fn draw(browser: &Browser, width: usize, height: usize) -> Self {
        let mut frame = Self {
            rows: Vec::new(),
            preview_height: 0,
            more: false,
        };
        let body = height.saturating_sub(2);
        let files = &browser.list.files;
        let help = match &browser.page {
            None => {
                frame.rows.push("Tracked files".into());
                let available = body.saturating_sub(usize::from(!browser.list.messages.is_empty()));
                if files.is_empty() {
                    frame.rows.push("No tracked source files".into());
                }
                let start = (browser.selected + 1).saturating_sub(available);
                for (i, (path, error)) in files.iter().enumerate().skip(start).take(available) {
                    let error = error.as_ref().map(|e| format!("  {e}")).unwrap_or_default();
                    frame
                        .rows
                        .push(format!("{} {path}{error}", cursor(i == browser.selected)));
                }
                if !browser.list.messages.is_empty() {
                    frame.rows.push(browser.list.messages.join(" | "));
                }
                "Up/Down select   Enter open   q quit"
            }
            Some(result) => {
                frame.rows.push(files[browser.selected].0.clone());
                match result {
                    Ok(page) => frame.outline(page, width, body),
                    Err(error) => frame.rows.push(error.clone()),
                }
                "Arrows navigate   [ ] siblings   PgUp/PgDn source   Esc files   q quit"
            }
        };
        frame.rows.truncate(height.saturating_sub(1));
        frame.rows.resize(height.saturating_sub(1), String::new());
        frame.rows.push(help.into());
        for row in &mut frame.rows {
            *row = clipped(row, width);
        }
        frame
    }

    fn outline(&mut self, page: &Page, width: usize, height: usize) {
        let tree = &page.tree;
        let tree_height = height / 2;
        let focus = tree
            .visible()
            .position(|i| i == tree.selected)
            .unwrap_or_default();
        let start = (focus + 1).saturating_sub(tree_height);
        for i in tree.visible().skip(start).take(tree_height) {
            let (depth, marker, label) = tree.label(i);
            let indent = " ".repeat(depth.saturating_mul(2).min(width / 2));
            self.rows.push(format!(
                "{} {indent}{marker} {label}",
                cursor(i == tree.selected)
            ));
        }
        self.rows.push("Source".into());
        self.preview_height = height.saturating_sub(self.rows.len().saturating_sub(1));
        let node = &page.document.outline.nodes[tree.selected];
        // Slice before decoding. Replacing an invalid UTF-8 byte can change subsequent byte offsets.
        let text = page
            .document
            .bytes
            .get(node.range.clone())
            .map(String::from_utf8_lossy)
            .unwrap_or(Cow::Borrowed("Invalid source range"));
        let mut rows = wrapped(&text, width).skip(page.scroll);
        self.rows.extend(rows.by_ref().take(self.preview_height));
        self.more = rows.next().is_some();
    }
}

fn cursor(selected: bool) -> char {
    if selected { '>' } else { ' ' }
}

/// Map project text to printable characters before measuring or writing it to the terminal.
fn display(ch: char) -> char {
    match ch {
        '\t' => ' ',
        c if c.is_control() => '�',
        c => c,
    }
}

fn clipped(text: &str, width: usize) -> String {
    let mut used = 0;
    text.chars()
        .map(display)
        .take_while(|c| {
            used += c.width().unwrap_or(0);
            used <= width
        })
        .collect()
}

fn wrapped(text: &str, width: usize) -> impl Iterator<Item = String> + '_ {
    let mut chars = text.chars().peekable();
    std::iter::from_fn(move || {
        chars.peek()?;
        let mut row = String::new();
        let mut used = 0;
        while let Some(&ch) = chars.peek() {
            if ch == '\n' {
                chars.next();
                break;
            }
            let ch = display(ch);
            let cells = ch.width().unwrap_or(0);
            if used + cells > width && !row.is_empty() {
                break;
            }
            chars.next();
            used += cells;
            row.push(ch);
        }
        Some(row)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_long_source_and_neutralizes_terminal_controls() {
        assert_eq!(
            wrapped("界ab\n\nx\x1b[2J", 3).collect::<Vec<_>>(),
            ["界a", "b", "", "x�[", "2J"]
        );
        assert_eq!(clipped("a\nb\x1bc", 4), "a�b�");
    }
}
