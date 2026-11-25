//! Lay out the file list, outline, and source preview in terminal cells.

use std::borrow::Cow;

use unicode_width::UnicodeWidthChar;

use crate::{Browser, Page, Screen};

pub(super) struct Frame {
    pub rows: Vec<String>,
    pub page_height: usize,
    pub more: bool,
}

impl Frame {
    pub fn draw(browser: &Browser, width: usize, height: usize) -> Self {
        let mut frame = Self {
            rows: Vec::new(),
            page_height: 0,
            more: false,
        };
        let keys: &[&str] = match &browser.screen {
            Screen::Files if !browser.list.messages.is_empty() => &[
                "Up/Down select",
                "Enter open",
                "PgUp/PgDn messages",
                "q quit",
            ],
            Screen::Files => &["Up/Down select", "Enter open", "q quit"],
            Screen::Outline(_) => &[
                "Arrows navigate",
                "[ ] siblings",
                "PgUp/PgDn source",
                "Esc files",
                "q quit",
            ],
            Screen::Unavailable { .. } => &["Esc files", "q quit"],
        };
        let footer = footer(keys, width, height);
        let content_height = height.saturating_sub(footer.len());
        let body = content_height.saturating_sub(1);
        let files = &browser.list.files;
        match &browser.screen {
            Screen::Files => frame.files(browser, width, body),
            Screen::Outline(page) => {
                frame.rows.push(files[browser.selected].0.clone());
                frame.outline(page, width, body);
            }
            Screen::Unavailable { message, .. } => {
                frame
                    .rows
                    .extend([files[browser.selected].0.clone(), message.clone()]);
            }
        }
        frame.rows.resize(content_height, String::new());
        frame.rows.extend(footer);
        for row in &mut frame.rows {
            *row = clipped(row, width);
        }
        frame
    }

    fn files(&mut self, browser: &Browser, width: usize, height: usize) {
        self.rows.push("Tracked files".into());
        let files = &browser.list.files;
        let messages = &browser.list.messages;
        let available = height.saturating_sub(usize::from(!messages.is_empty()));
        if files.is_empty() && available > 0 {
            self.rows.push("No tracked source files".into());
        }
        let start = (browser.selected + 1).saturating_sub(available);
        for (i, (path, error)) in files.iter().enumerate().skip(start).take(available) {
            let error = error.as_ref().map(|e| format!("  {e}")).unwrap_or_default();
            self.rows
                .push(format!("{} {path}{error}", cursor(i == browser.selected)));
        }
        self.page_height = height.saturating_sub(self.rows.len() - 1);
        let mut rows = messages
            .iter()
            .flat_map(|message| wrapped(message, width))
            .skip(browser.diagnostic_scroll);
        self.rows.extend(rows.by_ref().take(self.page_height));
        self.more = rows.next().is_some();
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
        self.page_height = height.saturating_sub(self.rows.len().saturating_sub(1));
        let node = &page.document.outline.nodes[tree.selected];
        // Slice before decoding. Replacing an invalid UTF-8 byte can change subsequent byte offsets.
        let text = page
            .document
            .bytes
            .get(node.range.clone())
            .map(String::from_utf8_lossy)
            .unwrap_or(Cow::Borrowed("Invalid source range"));
        let mut rows = wrapped(&text, width).skip(page.scroll);
        self.rows.extend(rows.by_ref().take(self.page_height));
        self.more = rows.next().is_some();
    }
}

/// Keep key/action pairs together where they fit and leave room for the selected content.
fn footer(keys: &[&str], width: usize, height: usize) -> Vec<String> {
    let max_rows = height.saturating_sub(2).max(1).min(height);
    let mut rows = Vec::new();
    let mut row = String::new();
    for &key in keys {
        if !row.is_empty() && row.len() + 3 + key.len() > width {
            rows.extend(wrapped(&row, width));
            row.clear();
        }
        if !row.is_empty() {
            row.push_str("   ");
        }
        row.push_str(key);
    }
    rows.extend(wrapped(&row, width));
    if rows.len() > max_rows {
        let compact = keys
            .iter()
            .rev()
            .filter_map(|hint| hint.rsplit_once(' ').map(|(keys, _)| keys))
            .collect::<Vec<_>>()
            .join(" ");
        return wrapped(&compact, width).take(max_rows).collect();
    }
    rows
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
    let mut source = text.chars().peekable();
    let mut chars = std::iter::from_fn(move || {
        let ch = source.next()?;
        Some(if ch == '\r' && source.next_if_eq(&'\n').is_some() {
            '\n'
        } else {
            ch
        })
    })
    .peekable();
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
    fn crlf_lines_wrap_without_showing_carriage_returns() {
        assert_eq!(
            wrapped("1234\r\n\r\nx\ry\nend\r", 4).collect::<Vec<_>>(),
            ["1234", "", "x�y", "end�"]
        );
    }

    #[test]
    fn wraps_long_source_and_neutralizes_terminal_controls() {
        assert_eq!(
            wrapped("界ab\n\nx\x1b[2J", 3).collect::<Vec<_>>(),
            ["界a", "b", "", "x�[", "2J"]
        );
        assert_eq!(clipped("a\nb\x1bc", 4), "a�b�");
    }
}
