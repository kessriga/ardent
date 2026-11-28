//! Draw the directory explorer, the open outline and its source on a Ratatui frame.

mod source;

use ardent_core::properties::Property;
use ardent_core::theme::{Rgb, Theme};
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
    let (status, diagnostic) = match browser.list.update_notice.as_deref() {
        Some(text) if text.len() <= usize::from(area.width) && area.height >= 8 => {
            (Some(text), None)
        }
        notice => (None, notice),
    };
    let hints = key_hints(browser, diagnostic.is_some());
    let mut footer = footer(hints.as_slice(), area.width.into(), area.height.into());
    if let Some(text) = status {
        footer.insert(0, text.to_owned());
    }
    let [content, help] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(footer.len() as u16)]).areas(area);
    frame.render_widget(Paragraph::new(footer.join("\n")), help);

    if area.width < 60 {
        return match browser.focus {
            Focus::Explorer => explorer(frame, content, browser, diagnostic),
            Focus::Document => document(frame, content, browser),
        };
    }
    let sidebar_width = (area.width / 3).min(36);
    let [sidebar, reader] =
        Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(1)]).areas(content);
    let divider = Block::default().borders(Borders::RIGHT);
    let inside = divider.inner(sidebar);
    frame.render_widget(divider, sidebar);
    let messages = explorer(frame, inside, browser, diagnostic);
    let reading = document(frame, reader, browser);
    match browser.focus {
        Focus::Explorer => messages,
        Focus::Document => reading,
    }
}

fn key_hints(browser: &Browser, has_notice: bool) -> Vec<&'static str> {
    let mut keys = match browser.focus {
        Focus::Explorer => vec!["Arrows navigate", "Enter open"],
        Focus::Document => match browser.screen() {
            Some(Screen::Outline(_)) => vec![
                "Arrows navigate",
                "[ ] siblings",
                "PgUp/PgDn source",
                "Tab properties",
            ],
            Some(Screen::Unavailable { .. }) => vec!["PgUp/PgDn error"],
            None => Vec::new(),
        },
    };
    if browser.focus == Focus::Explorer && (has_notice || browser.messages().next().is_some()) {
        keys.push("PgUp/PgDn messages");
    }
    if browser.open_path().is_some() {
        keys.push("Shift+Tab panes");
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

fn explorer(frame: &mut Frame<'_>, area: Rect, browser: &Browser, notice: Option<&str>) -> Paging {
    let area = heading(frame, area, "Explorer", browser.focus == Focus::Explorer);
    let entries = browser.explorer.rows();
    let list_height = entries.len().max(1).min(usize::from(area.height));
    let list_height = if browser.messages().next().is_none() && notice.is_none() {
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
    let diagnostics = notice
        .into_iter()
        .chain(browser.messages())
        .flat_map(|msg| wrapped(msg, messages.width.into()));
    page(
        frame,
        messages,
        diagnostics.skip(browser.diagnostic_scroll).map(Line::from),
    )
}

fn document(frame: &mut Frame<'_>, area: Rect, browser: &Browser) -> Paging {
    let focused = browser.focus == Focus::Document;
    let path = browser.open_path().map(printable);
    let title = path.map_or_else(|| "Outline".to_string(), |path| format!("Outline  {path}"));
    let area = heading(frame, area, &title, focused);
    match browser.screen() {
        Some(Screen::Outline(open)) => outline(frame, area, open, &browser.theme.colors),
        Some(Screen::Unavailable {
            message, scroll, ..
        }) => page(
            frame,
            area,
            wrapped(message, area.width.into())
                .skip(*scroll)
                .map(Line::from),
        ),
        None => {
            if !browser.list.files.is_empty() {
                frame.render_widget("Select a file and press Enter", area);
            }
            Paging::default()
        }
    }
}

fn outline(frame: &mut Frame<'_>, area: Rect, open: &Page, theme: &Theme) -> Paging {
    let [nodes, divider, source] = Layout::vertical([
        Constraint::Length(area.height / 2),
        Constraint::Length(u16::from(area.height >= 3)),
        Constraint::Min(0),
    ])
    .areas(area);
    let rows = || {
        open.tree.visible().flat_map(|index| {
            std::iter::once((index, None)).chain(
                open.visible_properties(index)
                    .map(move |property| (index, Some(property))),
            )
        })
    };
    let selected = open.tree.selected;
    let position = rows().take_while(|(index, _)| *index != selected).count();
    // Keep the selected label and as many of its open properties as fit in the viewport.
    let height = usize::from(nodes.height);
    let block_height = 1 + open.visible_properties(selected).count();
    let start = position.saturating_sub(height.saturating_sub(block_height));
    let visible = rows()
        .skip(start)
        .take(height)
        .map(|(index, property)| outline_line(open, index, property, nodes.width, theme));
    selected_list(frame, nodes, visible, position - start);
    frame.render_widget(Block::default().borders(Borders::TOP), divider);

    source::draw(frame, source, open, theme)
}

fn outline_line(
    open: &Page,
    index: usize,
    property: Option<&Property>,
    width: u16,
    theme: &Theme,
) -> Line<'static> {
    let (depth, marker) = open.tree.row_prefix(index);
    let width = usize::from(width);
    // The list reserves two cells for selection. Keep room for the property bullet.
    let max_indent = width.saturating_sub(3);
    let node_indent = depth
        .saturating_mul(2)
        .min(width / 2)
        .min(max_indent.saturating_sub(1));
    if let Some(property) = property {
        let indent = " ".repeat((node_indent + 2).min(max_indent));
        return Line::raw(format!(
            "{indent}⚬ {}: {}",
            printable(&property.name),
            printable(&property.value)
        ));
    }
    let node = &open.document.outline.nodes[index];
    let indent = " ".repeat(node_indent);
    Line::from(vec![
        Span::raw(format!("{indent}{marker}")),
        Span::styled(
            format!(" {}", printable(&node.label)),
            theme
                .outline
                .get(&node.kind)
                .map_or_else(|| kind_color(&node.kind), rgb),
        ),
    ])
}

fn rgb(&Rgb(r, g, b): &Rgb) -> Color {
    Color::Rgb(r, g, b)
}

fn kind_color(kind: &str) -> Color {
    match kind {
        "heading" | "mod" | "impl" | "list" => Color::Cyan,
        "struct" | "enum" | "trait" | "union" | "type" | "table" => Color::Yellow,
        "fn" | "macro" | "code" => Color::Green,
        "variant" | "const" | "static" | "quote" => Color::Magenta,
        "file" | "extern" | "front_matter" => Color::Blue,
        "paragraph" | "item" | "field" => Color::Reset,
        _ => Color::LightBlue,
    }
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
fn page(
    frame: &mut Frame<'_>,
    area: Rect,
    mut rows: impl Iterator<Item = Line<'static>>,
) -> Paging {
    let page_height = usize::from(area.height);
    let text: Vec<_> = rows.by_ref().take(page_height).collect();
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
    styled_rows(text, width, |_| Style::default()).map(|line| line.to_string())
}

fn styled_rows<'a>(
    text: &'a str,
    width: usize,
    mut style_at: impl FnMut(usize) -> Style + 'a,
) -> impl Iterator<Item = Line<'static>> + 'a {
    // Ratatui draws one extended grapheme per cell sequence. Wrap with the same width calculation.
    let mut graphemes = text.grapheme_indices(true).peekable();
    std::iter::from_fn(move || {
        if width == 0 {
            return None;
        }
        graphemes.peek()?;
        let mut row = Line::default();
        let mut used = 0;
        while let Some(&(offset, grapheme)) = graphemes.peek() {
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
            let style = style_at(offset);
            if let Some(last) = row.spans.last_mut()
                && (cells == 0 || last.style == style)
            {
                last.content.to_mut().push_str(symbol);
            } else {
                row.spans.push(Span::styled(symbol.to_owned(), style));
            }
        }
        Some(row)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ardent_core::highlight::{Category, Highlight};

    #[test]
    fn properties_stay_indented_when_the_node_depth_reaches_the_width_limit() -> std::io::Result<()>
    {
        use crate::Document;
        use ardent_core::outline::{Outline, ScannedNode};

        let mut children = Vec::new();
        for _ in 0..12 {
            children = vec![ScannedNode {
                kind: "section".into(),
                label: "section".into(),
                range: 0..1,
                label_ranges: vec![],
                children,
            }];
        }
        let page = Page::new(Document {
            outline: Outline::build("a", b"a", &children),
            properties: Default::default(),
            highlights: Default::default(),
            bytes: b"a".to_vec(),
        })?;
        let property = Property {
            name: "flag".into(),
            value: "true".into(),
            inline: true,
        };
        let theme = Theme::default();
        for (width, extra_indent) in [(40, 2), (7, 1)] {
            let branch = outline_line(&page, 10, None, width, &theme).to_string();
            let row = outline_line(&page, 10, Some(&property), width, &theme).to_string();
            let marker = branch.find('+').expect("collapsed branch");
            let bullet = row.find('⚬').expect("property marker");
            assert_eq!(bullet, marker + extra_indent);
            assert!(2 + bullet < usize::from(width));
        }
        Ok(())
    }

    #[test]
    fn a_four_row_terminal_keeps_the_selection_source_paging_and_focus_keys() {
        use crate::{Document, FileList};
        use ardent_core::outline::Outline;
        use crossterm::event::KeyCode;

        let list = FileList {
            files: vec![("a.md".into(), None)],
            messages: vec![],
            update_notice: None,
        };
        let mut browser = Browser::new(list);
        let load = |path: &str| {
            let bytes = b"first\nsecond".to_vec();
            Ok(Document {
                properties: Default::default(),
                highlights: Default::default(),
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
            update_notice: None,
        });
        let first_row = format!("{}❤️", "x".repeat(38));
        let text = format!("{first_row}Z");
        browser.enter(&|path| {
            Ok(Document {
                properties: Default::default(),
                highlights: Default::default(),
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
    fn wrapped_child_styles_follow_original_bytes_across_pages()
    -> Result<(), Box<dyn std::error::Error>> {
        use ardent_core::outline::{Outline, ScannedNode};
        use crossterm::event::KeyCode;
        use ratatui::{Terminal, backend::TestBackend};

        let mut bytes = b"skip\nA\xff\t".to_vec();
        let start = bytes.len();
        bytes.extend_from_slice("界e\u{301}xy\r\nz".as_bytes());
        let child = ScannedNode {
            kind: "heading".into(),
            label: "H".into(),
            range: 5..bytes.len(),
            label_ranges: vec![],
            children: vec![],
        };
        let mut open = Page::new(crate::Document {
            outline: Outline::build("a", &bytes, &[child]),
            properties: Default::default(),
            highlights: [Highlight {
                range: start..start + "界e\u{301}x".len(),
                category: Category::Code,
            }]
            .into(),
            bytes,
        })?;
        open.tree.navigate(KeyCode::Right);
        let mut terminal = Terminal::new(TestBackend::new(7, 4))?;
        terminal.draw(|frame| {
            outline(frame, frame.area(), &open, &Theme::default());
        })?;
        let cells = terminal.backend().buffer();
        assert_eq!(cells[(0, 1)].symbol(), ">");
        assert_eq!(cells[(4, 0)].fg, Color::Blue);
        assert_eq!(cells[(6, 1)].fg, Color::Cyan);
        assert!(cells[(6, 1)].modifier.contains(Modifier::BOLD));
        assert_eq!(cells[(1, 3)].symbol(), "�");
        for (x, symbol, color) in [
            (0, "A", Color::Reset),
            (2, " ", Color::Reset),
            (3, "界", Color::Green),
            (5, "e\u{301}", Color::Green),
            (6, "x", Color::Green),
        ] {
            assert_eq!((cells[(x, 3)].symbol(), cells[(x, 3)].fg), (symbol, color));
        }
        // Narrow the pane so that the last colored character moves onto the next page.
        terminal.backend_mut().resize(6, 4);
        open.scroll = 1;
        terminal.draw(|frame| {
            outline(frame, frame.area(), &open, &Theme::default());
        })?;
        let cells = terminal.backend().buffer();
        assert_eq!(
            (cells[(0, 3)].symbol(), cells[(0, 3)].fg),
            ("x", Color::Green)
        );
        assert_eq!(
            (cells[(1, 3)].symbol(), cells[(1, 3)].fg),
            ("y", Color::Reset)
        );
        open.scroll = 2;
        terminal.draw(|frame| {
            outline(frame, frame.area(), &open, &Theme::default());
        })?;
        assert_eq!(terminal.backend().buffer()[(0, 3)].symbol(), "z");
        Ok(())
    }

    #[test]
    fn a_long_name_keeps_its_scan_error_visible_in_the_sidebar() {
        use crate::FileList;

        let browser = Browser::new(FileList {
            files: vec![("long_module_name.rs".into(), Some("syntax error".into()))],
            messages: vec![],
            update_notice: None,
        });
        let rows = snapshot(&browser, 60, 14).0;
        let sidebar = rows[1].split('│').next().expect("sidebar");
        assert!(
            sidebar.contains('!') && sidebar.contains("long_module"),
            "{sidebar:?}"
        );
    }

    #[test]
    fn combining_accent_stays_on_a_substituted_tab_across_a_style_boundary() {
        let row = styled_rows("\t\u{301}x", 4, |offset| {
            if offset == 0 {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            }
        })
        .next()
        .expect("one row");
        assert_eq!(row.spans[0].content, " \u{301}");
        assert_eq!(row.spans[1].content, "x");
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
