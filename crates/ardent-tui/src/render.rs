//! Draw the directory explorer, the open outline and its source on a Ratatui frame.

use std::borrow::Cow;

use ratatui::Frame;
use ratatui::buffer::CellWidth;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListState, Paragraph};
use unicode_segmentation::UnicodeSegmentation;

use crate::{Browser, Focus, Page, Screen};

/// The number of wrapped rows the focused pane can page through on this frame.
#[derive(Default)]
pub(super) struct Paging {
    pub page_height: usize,
    pub more: bool,
}

pub(super) fn draw(frame: &mut Frame<'_>, browser: &Browser) -> Paging {
    let area = frame.area();
    let hints = key_hints(browser);
    let footer = footer(hints.as_slice(), area.width.into(), area.height.into());
    let [content, help] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(footer.len() as u16)]).areas(area);
    frame.render_widget(Paragraph::new(footer.join("\n")), help);

    if area.width < 60 {
        return match browser.focus {
            Focus::Explorer => explorer(frame, content, browser),
            Focus::Document => document(frame, content, browser),
        };
    }
    let sidebar_width = (area.width / 3).min(36);
    let [sidebar, reader] =
        Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(1)]).areas(content);
    let divider = Block::default().borders(Borders::RIGHT);
    let inside = divider.inner(sidebar);
    frame.render_widget(divider, sidebar);
    let messages = explorer(frame, inside, browser);
    let reading = document(frame, reader, browser);
    match browser.focus {
        Focus::Explorer => messages,
        Focus::Document => reading,
    }
}

fn key_hints(browser: &Browser) -> Vec<&'static str> {
    let mut keys = match browser.focus {
        Focus::Explorer => vec!["Arrows navigate", "Enter open"],
        Focus::Document => match browser.screen() {
            Some(Screen::Outline(_)) => vec!["Arrows navigate", "[ ] siblings", "PgUp/PgDn source"],
            Some(Screen::Unavailable { .. }) => vec!["PgUp/PgDn error"],
            None => Vec::new(),
        },
    };
    if browser.focus == Focus::Explorer && !browser.list.messages.is_empty() {
        keys.push("PgUp/PgDn messages");
    }
    if browser.open_path().is_some() {
        keys.push("Tab panes");
        keys.push("Esc close");
    }
    keys.push("q quit");
    keys
}

/// The active pane has a text marker as well as a highlighted heading.
fn heading(frame: &mut Frame<'_>, area: Rect, title: &str, focused: bool) -> Rect {
    let [header, body] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let style = Style::default().add_modifier(Modifier::BOLD);
    let line = if focused {
        Line::styled(format!("> {title}"), style.fg(Color::Cyan))
    } else {
        Line::styled(title, style)
    };
    frame.render_widget(line, header);
    body
}

fn explorer(frame: &mut Frame<'_>, area: Rect, browser: &Browser) -> Paging {
    let area = heading(frame, area, "Explorer", browser.focus == Focus::Explorer);
    let entries = browser.explorer.rows();
    let list_height = entries.len().max(1).min(usize::from(area.height));
    let list_height = if browser.list.messages.is_empty() {
        list_height
    } else {
        // Keep at least one row for messages, even when the explorer fills the pane.
        list_height.min(usize::from(area.height.saturating_sub(1)))
    };
    let [files, messages] =
        Layout::vertical([Constraint::Length(list_height as u16), Constraint::Min(0)]).areas(area);
    let selected = browser.explorer.selected();
    let start = (selected + 1).saturating_sub(list_height);
    if entries.is_empty() {
        frame.render_widget("No tracked source files", files);
    } else {
        let items = entries.iter().skip(start).take(list_height).map(|entry| {
            let name = entry.path.rsplit('/').next().unwrap_or(&entry.path);
            let indent = " ".repeat(
                entry
                    .depth
                    .saturating_mul(2)
                    .min(usize::from(files.width) / 2),
            );
            let name = printable(name);
            let label = if entry.directory {
                let marker = if browser.explorer.is_expanded(&entry.path) {
                    '▾'
                } else {
                    '▸'
                };
                format!("{indent}{marker} {name}/")
            } else {
                let failed = browser
                    .list
                    .file(&entry.path)
                    .is_some_and(|(_, error)| error.is_some());
                let marker = if failed { "! " } else { "" };
                format!("{indent}{marker}{name}")
            };
            if browser.open_path() == Some(&entry.path) {
                Line::styled(label, Style::default().add_modifier(Modifier::UNDERLINED))
            } else {
                Line::from(label)
            }
        });
        selected_list(frame, files, items, selected.saturating_sub(start));
    }
    let diagnostics = browser
        .list
        .messages
        .iter()
        .flat_map(|msg| wrapped(msg, messages.width.into()));
    page(frame, messages, diagnostics.skip(browser.diagnostic_scroll))
}

fn document(frame: &mut Frame<'_>, area: Rect, browser: &Browser) -> Paging {
    let focused = browser.focus == Focus::Document;
    let path = browser.open_path().map(printable);
    let title = path.map_or_else(|| "Outline".to_string(), |path| format!("Outline  {path}"));
    let area = heading(frame, area, &title, focused);
    match browser.screen() {
        Some(Screen::Outline(open)) => outline(frame, area, open),
        Some(Screen::Unavailable {
            message, scroll, ..
        }) => page(
            frame,
            area,
            wrapped(message, area.width.into()).skip(*scroll),
        ),
        None => {
            if !browser.list.files.is_empty() {
                frame.render_widget("Select a file and press Enter", area);
            }
            Paging::default()
        }
    }
}

fn outline(frame: &mut Frame<'_>, area: Rect, open: &Page) -> Paging {
    let [nodes, title, source] = Layout::vertical([
        Constraint::Length(area.height / 2),
        Constraint::Length(u16::from(area.height >= 3)),
        Constraint::Min(0),
    ])
    .areas(area);
    let tree = &open.tree;
    let selected = tree
        .visible()
        .position(|i| i == tree.selected)
        .unwrap_or_default();
    let start = (selected + 1).saturating_sub(usize::from(nodes.height));
    let visible = tree
        .visible()
        .skip(start)
        .take(nodes.height.into())
        .map(|i| {
            let (depth, marker, label) = tree.label(i);
            let indent = " ".repeat(depth.saturating_mul(2).min(usize::from(nodes.width) / 2));
            Line::from(format!("{indent}{marker} {}", printable(label)))
        });
    selected_list(frame, nodes, visible, selected.saturating_sub(start));
    frame.render_widget(
        Span::styled("Source", Style::default().add_modifier(Modifier::BOLD)),
        title,
    );

    let node = &open.document.outline.nodes[tree.selected];
    // Slice before decoding because replacing an invalid UTF-8 byte can move later offsets.
    let text = open
        .document
        .bytes
        .get(node.range.clone())
        .map(String::from_utf8_lossy)
        .unwrap_or(Cow::Borrowed("Invalid source range"));
    page(
        frame,
        source,
        wrapped(&text, source.width.into()).skip(open.scroll),
    )
}

fn selected_list(
    frame: &mut Frame<'_>,
    area: Rect,
    items: impl Iterator<Item = Line<'static>>,
    selected: usize,
) {
    let list = List::new(items)
        .highlight_symbol("> ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_stateful_widget(
        list,
        area,
        &mut ListState::default().with_selected(Some(selected)),
    );
}

/// Only decode and hold the rows that reach the viewport, plus one to decide whether Page Down can advance.
fn page(frame: &mut Frame<'_>, area: Rect, mut rows: impl Iterator<Item = String>) -> Paging {
    let page_height = usize::from(area.height);
    let text: Vec<_> = rows.by_ref().take(page_height).map(Line::from).collect();
    frame.render_widget(Paragraph::new(text), area);
    Paging {
        page_height,
        more: rows.next().is_some(),
    }
}

fn printable(text: &str) -> String {
    text.chars().map(display).collect()
}

#[cfg(test)]
pub(super) fn snapshot(browser: &Browser, width: u16, height: u16) -> (Vec<String>, Paging) {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("in-memory terminal");
    let mut paging = Paging::default();
    terminal
        .draw(|frame| paging = draw(frame, browser))
        .expect("in-memory render");
    let buffer = terminal.backend().buffer();
    let rows = (0..height)
        .map(|y| {
            let row: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
            row.trim_end().to_owned()
        })
        .collect();
    (rows, paging)
}

/// Keep key/action pairs together where they fit and leave room for the selected content.
fn footer(keys: &[&str], width: usize, height: usize) -> Vec<String> {
    // A short terminal still needs a heading, a selected node and a source row.
    let max_rows = height.saturating_sub(3).max(1).min(height);
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
        let (full, short) = if let Some((first, rest)) = keys.split_first()
            && max_rows > 1
            && first.len() <= width
        {
            (Some(first.to_string()), rest)
        } else {
            (None, keys)
        };
        let compact = short
            .iter()
            .rev()
            .filter_map(|hint| hint.rsplit_once(' ').map(|(keys, _)| keys))
            .collect::<Vec<_>>()
            .join(" ");
        return full
            .into_iter()
            .chain(wrapped(&compact, width))
            .take(max_rows)
            .collect();
    }
    rows
}

/// Map project text to printable characters before measuring or writing it to the terminal.
fn display(ch: char) -> char {
    match ch {
        '\t' => ' ',
        c if c.is_control() => '�',
        c => c,
    }
}

fn wrapped(text: &str, width: usize) -> impl Iterator<Item = String> + '_ {
    // Ratatui draws one extended grapheme per cell sequence. Wrap with the same width calculation.
    let mut graphemes = text.graphemes(true).peekable();
    std::iter::from_fn(move || {
        if width == 0 {
            return None;
        }
        graphemes.peek()?;
        let mut row = String::new();
        let mut used = 0;
        while let Some(&grapheme) = graphemes.peek() {
            let printable = match grapheme {
                "\n" | "\r\n" => {
                    graphemes.next();
                    break;
                }
                "\t" => " ",
                control if control.starts_with(char::is_control) => "�",
                other => other,
            };
            let cells = usize::from(printable.cell_width());
            // In a pane narrower than a single glyph, show a visible placeholder.
            let (symbol, cells) = if cells > width {
                ("�", 1)
            } else {
                (printable, cells)
            };
            if used + cells > width {
                break;
            }
            graphemes.next();
            used += cells;
            row.push_str(symbol);
        }
        Some(row)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_four_row_terminal_keeps_the_selection_source_paging_and_focus_keys() {
        use crate::{Document, FileList};
        use ardent_core::outline::Outline;
        use crossterm::event::KeyCode;

        let list = FileList {
            files: vec![("a.md".into(), None)],
            messages: vec![],
        };
        let mut browser = Browser::new(list);
        let load = |path: &str| {
            let bytes = b"first\nsecond".to_vec();
            Ok(Document {
                outline: Outline::build(path, &bytes, &[]),
                bytes,
            })
        };
        browser.enter(&load);
        let (rows, paging) = snapshot(&browser, 24, 4);
        assert!(
            rows[1].contains("> ") && rows[1].contains("a.md"),
            "{rows:?}"
        );
        assert_eq!(rows[2], "first");
        assert!(rows[3].contains('q') && rows[3].contains("Tab"));
        assert_eq!(paging.page_height, 1);
        assert!(paging.more);
        browser.key(KeyCode::PageDown, paging.page_height, paging.more, &load);
        assert_eq!(snapshot(&browser, 24, 4).0[2], "second");
    }

    #[test]
    fn a_two_cell_emoji_at_the_right_edge_keeps_the_following_text() {
        use crate::{Document, FileList};
        use ardent_core::outline::Outline;

        let mut browser = Browser::new(FileList {
            files: vec![("a.md".into(), None)],
            messages: vec![],
        });
        let first_row = format!("{}❤️", "x".repeat(38));
        let text = format!("{first_row}Z");
        browser.enter(&|path| {
            Ok(Document {
                outline: Outline::build(path, text.as_bytes(), &[]),
                bytes: text.as_bytes().to_vec(),
            })
        });
        let rows = snapshot(&browser, 40, 14).0;
        assert!(
            rows.windows(2)
                .any(|pair| pair[0] == first_row && pair[1] == "Z"),
            "{rows:?}"
        );
    }

    #[test]
    fn a_long_name_keeps_its_scan_error_visible_in_the_sidebar() {
        use crate::FileList;

        let browser = Browser::new(FileList {
            files: vec![("long_module_name.rs".into(), Some("syntax error".into()))],
            messages: vec![],
        });
        let rows = snapshot(&browser, 60, 14).0;
        let sidebar = rows[1].split('│').next().expect("sidebar");
        assert!(
            sidebar.contains('!') && sidebar.contains("long_module"),
            "{sidebar:?}"
        );
    }

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
        assert_eq!(printable("a\nb\x1bc"), "a�b�c");
        assert_eq!(wrapped("A👩‍💻B", 2).collect::<Vec<_>>(), ["A", "👩‍💻", "B"]);
        assert_eq!(wrapped("界a", 1).collect::<Vec<_>>(), ["�", "a"]);
    }
}
