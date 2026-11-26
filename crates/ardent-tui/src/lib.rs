//! Keyboard navigation and terminal display for the outline browser.

use std::{io, mem};

use ardent_core::outline::Outline;
use crossterm::event::KeyCode;

mod render;
mod terminal;
mod tree;

pub use terminal::require_terminal;
use tree::Tree;

/// The file picker contents from one sync.
pub struct FileList {
    /// Project-relative source paths in ascending string order, each paired with its scan error, if any.
    /// The browser uses this order to find the selected file again after a refresh.
    pub files: Vec<(String, Option<String>)>,
    /// Project-wide errors and warnings to display alongside the file list.
    pub messages: Vec<String>,
}

/// A snapshot for browsing one tracked source file.
pub struct Document {
    /// Nodes in document order, starting with the file node. Their byte ranges index [`Self::bytes`].
    pub outline: Outline,
    /// Source bytes whose content hash matches the structure stamp used to load the outline.
    pub bytes: Vec<u8>,
}

struct Page {
    document: Document,
    tree: Tree,
    /// Offset in wrapped terminal rows within the selected node's source range.
    scroll: usize,
}

impl Page {
    fn new(document: Document) -> io::Result<Self> {
        let tree = Tree::new(&document.outline)?;
        Ok(Self {
            document,
            tree,
            scroll: 0,
        })
    }

    fn restore(&mut self, previous: &Self) {
        self.tree.restore(
            &self.document.outline,
            &previous.tree,
            &previous.document.outline,
        );
        let node = &self.document.outline.nodes[self.tree.selected];
        let old = &previous.document.outline.nodes[previous.tree.selected];
        // A node can move within the file while the same wrapped passage remains on screen.
        if node.path == old.path
            && let Some(text) = self.document.bytes.get(node.range.clone())
            && Some(text) == previous.document.bytes.get(old.range.clone())
        {
            self.scroll = previous.scroll;
        }
    }
}

enum Screen {
    Files,
    Outline(Page),
    /// Keep the last good page for navigation recovery. Only the diagnostic is displayed.
    Unavailable {
        message: String,
        scroll: usize,
        previous: Option<Page>,
    },
}

struct Browser {
    list: FileList,
    selected: usize,
    screen: Screen,
    diagnostic_scroll: usize,
}

impl Browser {
    fn open_path(&self) -> Option<&str> {
        match self.screen {
            Screen::Files => None,
            Screen::Outline(_) | Screen::Unavailable { .. } => self
                .list
                .files
                .get(self.selected)
                .map(|(path, _)| path.as_str()),
        }
    }

    fn open(&mut self, load: &impl Fn(&str) -> io::Result<Document>) {
        let Some((path, error)) = self.list.files.get(self.selected) else {
            return;
        };
        let result = match error {
            Some(error) => Err(error.clone()),
            None => load(path)
                .and_then(Page::new)
                .map_err(|err| err.to_string()),
        };
        if let (Err(error), Screen::Unavailable { message, .. }) = (&result, &self.screen)
            && error == message
        {
            return;
        }
        let previous = match mem::replace(&mut self.screen, Screen::Files) {
            Screen::Files => None,
            Screen::Outline(page) => Some(page),
            Screen::Unavailable { previous, .. } => previous,
        };
        self.screen = match result {
            Ok(mut page) => {
                if let Some(previous) = previous {
                    page.restore(&previous);
                }
                Screen::Outline(page)
            }
            Err(message) => Screen::Unavailable {
                message,
                scroll: 0,
                previous,
            },
        };
    }

    fn refresh(&mut self, list: FileList, load: &impl Fn(&str) -> io::Result<Document>) {
        let current = self.list.files.get(self.selected).map(|(path, _)| path);
        let index = current.and_then(|path| list.files.binary_search_by(|(p, _)| p.cmp(path)).ok());
        if self.list.messages != list.messages {
            self.diagnostic_scroll = 0;
        }
        self.list = list;
        self.selected =
            index.unwrap_or_else(|| self.selected.min(self.list.files.len().saturating_sub(1)));
        if index.is_none() {
            self.screen = Screen::Files;
        } else if !matches!(self.screen, Screen::Files) {
            self.open(load);
        }
    }

    fn key(
        &mut self,
        key: KeyCode,
        height: usize,
        more: bool,
        load: &impl Fn(&str) -> io::Result<Document>,
    ) {
        match (&mut self.screen, key) {
            (_, KeyCode::Esc) => self.screen = Screen::Files,
            (Screen::Files, KeyCode::Enter) => self.open(load),
            (screen, KeyCode::PageUp | KeyCode::PageDown) => {
                let scroll = match screen {
                    Screen::Files => &mut self.diagnostic_scroll,
                    Screen::Outline(page) => &mut page.scroll,
                    Screen::Unavailable { scroll, .. } => scroll,
                };
                if key == KeyCode::PageUp {
                    *scroll = scroll.saturating_sub(height);
                } else if more {
                    *scroll += height;
                }
            }
            (Screen::Files, KeyCode::Up) => self.selected = self.selected.saturating_sub(1),
            (Screen::Files, KeyCode::Down) => {
                self.selected = (self.selected + 1).min(self.list.files.len().saturating_sub(1))
            }
            (Screen::Outline(page), key) => {
                let old = page.tree.selected;
                page.tree.navigate(key);
                if old != page.tree.selected {
                    page.scroll = 0;
                }
            }
            _ => {}
        }
    }
}

/// Opens the file picker on an interactive terminal using the result of the initial sync.
///
/// `load` receives a project-relative path when the reader opens a file or a refresh reloads it.
/// Supply a [`Document`] from the same source revision as its structure. A load error appears
/// in place of that file's outline, and the reader can return to the picker.
///
/// The event loop passes the open source path to `refresh`, including while a scan error is
/// displayed. The picker passes `None`. Return `Ok(None)` promptly while idle and
/// `Ok(Some(list))` with the complete replacement [`FileList`] after a sync.
///
/// # Errors
///
/// Returns an error if the terminal cannot be used or `refresh` fails.
pub fn run(
    list: FileList,
    load: impl Fn(&str) -> io::Result<Document>,
    refresh: impl FnMut(Option<&str>) -> io::Result<Option<FileList>>,
) -> io::Result<()> {
    let browser = Browser {
        list,
        selected: 0,
        screen: Screen::Files,
        diagnostic_scroll: 0,
    };
    terminal::run(browser, load, refresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Frame;
    use ardent_core::outline::ScannedNode;

    fn files(error: Option<&str>) -> FileList {
        FileList {
            files: vec![("notes.md".into(), error.map(str::to_owned))],
            messages: Vec::new(),
        }
    }

    fn document(_: &str) -> io::Result<Document> {
        let text: String = (0..30).map(|n| format!("line {n}\n")).collect();
        let entry = ScannedNode {
            kind: "paragraph".into(),
            label: "passage".into(),
            range: 0..text.len(),
            label_ranges: vec![],
            children: vec![],
        };
        Ok(Document {
            outline: Outline::build("notes.md", text.as_bytes(), &[entry]),
            bytes: text.into_bytes(),
        })
    }

    fn reading_passage() -> Browser {
        let mut browser = Browser {
            list: files(None),
            selected: 0,
            screen: Screen::Files,
            diagnostic_scroll: 0,
        };
        browser.key(KeyCode::Enter, 5, false, &document);
        browser.key(KeyCode::Right, 5, false, &document);
        browser.key(KeyCode::PageDown, 5, true, &document);
        browser
    }

    #[test]
    fn file_picker_pages_through_wrapped_diagnostics() {
        let mut browser = reading_passage();
        browser.key(KeyCode::Esc, 0, false, &document);
        browser.list.messages = vec![
            "a".repeat(61),
            "second warning".into(),
            "last warning".into(),
        ];
        let first = Frame::draw(&browser, 30, 7);
        assert!(first.rows.iter().any(|row| row == "> notes.md"));
        assert!(first.rows.iter().any(|row| row == &"a".repeat(30)));
        assert!(
            first
                .rows
                .iter()
                .any(|row| row.contains("PgUp/PgDn messages"))
        );
        assert!(first.more);
        browser.key(KeyCode::PageDown, first.page_height, first.more, &document);
        let last = Frame::draw(&browser, 30, 7);
        assert!(!last.more);
        assert!(last.rows.iter().any(|row| row == "second warning"));
        assert!(last.rows.iter().any(|row| row == "last warning"));
        browser.key(KeyCode::PageUp, last.page_height, last.more, &document);
        assert_eq!(Frame::draw(&browser, 30, 7).rows, first.rows);

        browser.key(KeyCode::PageDown, first.page_height, first.more, &document);
        let mut changed = files(None);
        changed.messages.push("new warning".into());
        browser.refresh(changed, &document);
        assert!(
            Frame::draw(&browser, 30, 7)
                .rows
                .iter()
                .any(|row| row == "new warning")
        );
    }

    #[test]
    fn file_error_pages_survive_refresh_until_the_message_changes() {
        let mut browser = reading_passage();
        let message = format!("{}\nline 9, column 7", "x".repeat(44));
        browser.refresh(files(Some(&message)), &document);
        let first = Frame::draw(&browser, 22, 5);
        assert!(first.rows.iter().any(|row| row == &"x".repeat(22)));
        assert!(first.rows.iter().any(|row| row.contains("PgUp/PgDn error")));
        assert!(first.more);

        browser.key(KeyCode::PageDown, first.page_height, first.more, &document);
        let last = Frame::draw(&browser, 22, 5);
        assert!(last.rows.iter().any(|row| row == "line 9, column 7"));
        assert!(!last.more);
        browser.refresh(files(Some(&message)), &document);
        assert_eq!(Frame::draw(&browser, 22, 5).rows, last.rows);
        browser.key(KeyCode::PageUp, last.page_height, last.more, &document);
        assert_eq!(Frame::draw(&browser, 22, 5).rows, first.rows);

        browser.key(KeyCode::PageDown, first.page_height, first.more, &document);
        browser.refresh(files(Some("new error")), &document);
        assert!(
            Frame::draw(&browser, 22, 5)
                .rows
                .iter()
                .any(|row| row == "new error")
        );
    }

    #[test]
    fn narrow_outline_shows_navigation_and_exit_keys() {
        let browser = reading_passage();
        let rows = Frame::draw(&browser, 24, 12).rows;
        for hint in [
            "Arrows navigate",
            "[ ] siblings",
            "PgUp/PgDn source",
            "Esc files",
            "q quit",
        ] {
            assert!(
                rows.iter().any(|row| row.contains(hint)),
                "{hint:?} missing from {rows:?}"
            );
        }
        assert!(rows.iter().any(|row| row.contains("passage")));
        assert!(rows.iter().any(|row| row == "line 5"));
        assert_eq!(Frame::draw(&browser, 1, 1).rows, ["q"]);
    }

    #[test]
    fn unrelated_refresh_preserves_the_visible_passage() {
        let mut browser = reading_passage();
        let before = Frame::draw(&browser, 80, 12).rows;
        assert!(before.iter().any(|row| row == "line 5"));
        browser.refresh(files(None), &document);
        assert_eq!(Frame::draw(&browser, 80, 12).rows, before);
    }

    #[test]
    fn changing_the_selected_passage_resets_its_scroll() {
        let mut browser = reading_passage();
        browser.refresh(files(None), &|path| {
            let mut changed = document(path)?;
            changed.bytes[0] = b'L';
            Ok(changed)
        });
        let rows = Frame::draw(&browser, 80, 12).rows;
        assert!(rows.iter().any(|row| row == "Line 0"));
        assert!(
            rows.iter()
                .any(|row| row.starts_with('>') && row.contains("passage"))
        );
    }

    #[test]
    fn repair_restores_selection_without_exposing_stale_text() {
        let mut browser = reading_passage();
        let before = Frame::draw(&browser, 80, 12).rows;
        for _ in 0..2 {
            browser.refresh(files(Some("syntax error")), &document);
            let error = Frame::draw(&browser, 80, 12).rows;
            assert!(error.iter().any(|row| row == "syntax error"));
            assert!(!error.iter().any(|row| row.contains("line 5")));
        }
        browser.refresh(files(None), &document);
        assert_eq!(Frame::draw(&browser, 80, 12).rows, before);
    }
}
