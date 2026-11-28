//! Keyboard navigation and terminal display for the outline browser.

use std::io;
use std::sync::Arc;

use ardent_core::highlight::Highlight;
use ardent_core::outline::Outline;
use ardent_core::theme::Theme;
use crossterm::event::KeyCode;

mod explorer;
mod render;
mod terminal;
mod tree;

use explorer::Explorer;
pub use terminal::require_terminal;
use tree::Tree;

/// Tracked source files and project diagnostics from one sync.
pub struct FileList {
    /// Project-relative source paths in ascending string order, each paired with its scan error, if any.
    /// The browser uses this order to find the selected file again after a refresh.
    pub files: Vec<(String, Option<String>)>,
    /// Project-wide errors and warnings to display alongside the file list.
    pub messages: Vec<String>,
    /// Available release plugin updates and the command to install them.
    pub update_notice: Option<String>,
}

/// Colors and any diagnostic from loading the selected palette.
#[derive(Clone, Default)]
pub struct BrowserTheme {
    pub colors: Theme,
    pub diagnostic: Option<String>,
}

/// Project sync and theme reloads can finish independently.
pub enum BrowserUpdate {
    Files(FileList),
    Theme(BrowserTheme),
}

impl FileList {
    fn file(&self, path: &str) -> Option<&(String, Option<String>)> {
        let index = self
            .files
            .binary_search_by(|(p, _)| p.as_str().cmp(path))
            .ok()?;
        self.files.get(index)
    }
}

/// A snapshot for browsing one tracked source file.
pub struct Document {
    /// Sorted, disjoint syntax spans in these source bytes. An empty list displays plain text.
    pub highlights: Arc<[Highlight]>,
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
    Outline(Page),
    /// Keep the last good page for navigation recovery. Only the diagnostic is displayed.
    Unavailable {
        message: String,
        scroll: usize,
        previous: Option<Page>,
    },
}

struct OpenFile {
    path: String,
    screen: Screen,
}

impl Screen {
    fn load(
        path: &str,
        error: Option<&str>,
        load: &impl Fn(&str) -> io::Result<Document>,
        previous: Option<Self>,
    ) -> Self {
        let result = match error {
            Some(message) => Err(message.to_owned()),
            None => load(path)
                .and_then(Page::new)
                .map_err(|err| err.to_string()),
        };
        let previous = match previous {
            Some(Screen::Unavailable {
                message,
                scroll,
                previous,
            }) => {
                if result.as_ref().err() == Some(&message) {
                    return Screen::Unavailable {
                        message,
                        scroll,
                        previous,
                    };
                }
                previous
            }
            Some(Screen::Outline(page)) => Some(page),
            None => None,
        };
        match result {
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
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Explorer,
    Document,
}

struct Browser {
    theme: BrowserTheme,
    list: FileList,
    explorer: Explorer,
    /// The reader can move the explorer cursor while keeping another file open.
    opened: Option<OpenFile>,
    focus: Focus,
    diagnostic_scroll: usize,
}

impl Browser {
    fn new(list: FileList) -> Self {
        Self {
            theme: BrowserTheme::default(),
            explorer: Explorer::new(&list.files),
            list,
            opened: None,
            focus: Focus::Explorer,
            diagnostic_scroll: 0,
        }
    }

    fn open_path(&self) -> Option<&str> {
        self.opened.as_ref().map(|file| file.path.as_str())
    }

    fn screen(&self) -> Option<&Screen> {
        self.opened.as_ref().map(|file| &file.screen)
    }

    fn close(&mut self) {
        self.opened = None;
        self.focus = Focus::Explorer;
    }

    fn messages(&self) -> impl Iterator<Item = &str> {
        self.theme
            .diagnostic
            .iter()
            .chain(&self.list.messages)
            .map(String::as_str)
    }

    fn apply(&mut self, update: BrowserUpdate, load: &impl Fn(&str) -> io::Result<Document>) {
        match update {
            BrowserUpdate::Files(files) => self.refresh(files, load),
            BrowserUpdate::Theme(theme) => {
                if self.theme.diagnostic != theme.diagnostic {
                    self.diagnostic_scroll = 0;
                }
                self.theme = theme;
            }
        }
    }

    fn refresh(&mut self, list: FileList, load: &impl Fn(&str) -> io::Result<Document>) {
        if self.list.messages != list.messages || self.list.update_notice != list.update_notice {
            self.diagnostic_scroll = 0;
        }
        if self
            .list
            .files
            .iter()
            .map(|(path, _)| path)
            .ne(list.files.iter().map(|(path, _)| path))
        {
            self.explorer.refresh(&list.files);
        }
        self.list = list;
        if let Some(mut file) = self.opened.take() {
            match self.list.file(&file.path) {
                Some((path, error)) => {
                    file.screen = Screen::load(path, error.as_deref(), load, Some(file.screen));
                    self.opened = Some(file);
                }
                None => self.close(),
            }
        }
    }

    fn enter(&mut self, load: &impl Fn(&str) -> io::Result<Document>) {
        let Some(entry) = self.explorer.selected_entry() else {
            return;
        };
        if entry.directory {
            self.explorer.navigate(KeyCode::Enter);
            return;
        }
        if let Some((path, error)) = self.list.file(&entry.path) {
            self.opened = Some(OpenFile {
                path: path.clone(),
                screen: Screen::load(path, error.as_deref(), load, None),
            });
            self.focus = Focus::Document;
        }
    }

    fn resized(&mut self) {
        self.diagnostic_scroll = 0;
        if let Some(file) = &mut self.opened {
            match &mut file.screen {
                Screen::Outline(page) => page.scroll = 0,
                Screen::Unavailable {
                    scroll, previous, ..
                } => {
                    *scroll = 0;
                    if let Some(page) = previous {
                        page.scroll = 0;
                    }
                }
            }
        }
    }

    fn key(
        &mut self,
        key: KeyCode,
        height: usize,
        more: bool,
        load: &impl Fn(&str) -> io::Result<Document>,
    ) {
        match key {
            KeyCode::Esc => self.close(),
            KeyCode::Tab | KeyCode::BackTab if self.opened.is_some() => {
                self.focus = match self.focus {
                    Focus::Explorer => Focus::Document,
                    Focus::Document => Focus::Explorer,
                };
            }
            KeyCode::Enter if self.focus == Focus::Explorer => self.enter(load),
            KeyCode::PageUp | KeyCode::PageDown => {
                let scroll = match (&mut self.opened, self.focus) {
                    (Some(file), Focus::Document) => match &mut file.screen {
                        Screen::Outline(page) => &mut page.scroll,
                        Screen::Unavailable { scroll, .. } => scroll,
                    },
                    _ => &mut self.diagnostic_scroll,
                };
                if key == KeyCode::PageUp {
                    *scroll = scroll.saturating_sub(height);
                } else if more {
                    *scroll = scroll.saturating_add(height);
                }
            }
            key if self.focus == Focus::Explorer => self.explorer.navigate(key),
            key => {
                if let Some(OpenFile {
                    screen: Screen::Outline(page),
                    ..
                }) = &mut self.opened
                {
                    let old = page.tree.selected;
                    page.tree.navigate(key);
                    if old != page.tree.selected {
                        page.scroll = 0;
                    }
                }
            }
        }
    }
}

/// Opens the directory explorer on an interactive terminal using the result of the initial sync.
///
/// `load` receives a project-relative path when the reader opens a file or a refresh reloads it.
/// Supply a [`Document`] from the same source revision as its structure. A load error appears
/// in place of that file's outline, and the reader can still navigate the explorer.
///
/// The event loop passes the open source path to `refresh`, including while a scan error is
/// displayed or the explorer has focus. A closed document passes `None`. Return `Ok(None)` promptly while idle and
/// a [`BrowserUpdate`] when a sync finishes, settings fail to load, or the active palette changes.
///
/// # Errors
///
/// Returns an error if the terminal cannot be used or `refresh` fails.
pub fn run(
    list: FileList,
    theme: BrowserTheme,
    load: impl Fn(&str) -> io::Result<Document>,
    refresh: impl FnMut(Option<&str>) -> io::Result<Option<BrowserUpdate>>,
) -> io::Result<()> {
    let mut browser = Browser::new(list);
    browser.theme = theme;
    terminal::run(browser, load, refresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::snapshot;
    use ardent_core::outline::ScannedNode;

    fn files(error: Option<&str>) -> FileList {
        FileList {
            files: vec![("notes.md".into(), error.map(str::to_owned))],
            messages: Vec::new(),
            update_notice: None,
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
            highlights: Default::default(),
            outline: Outline::build("notes.md", text.as_bytes(), &[entry]),
            bytes: text.into_bytes(),
        })
    }

    fn reading_passage() -> Browser {
        let mut browser = Browser::new(files(None));
        browser.key(KeyCode::Enter, 5, false, &document);
        browser.key(KeyCode::Right, 5, false, &document);
        browser.key(KeyCode::PageDown, 5, true, &document);
        browser
    }

    #[test]
    fn theme_and_config_updates_keep_the_scrolled_page_without_reloading() {
        let mut browser = reading_passage();
        let load = |_: &str| Err(io::Error::other("unexpected document reload"));
        browser.apply(
            BrowserUpdate::Theme(BrowserTheme {
                diagnostic: Some("invalid selector".into()),
                ..BrowserTheme::default()
            }),
            &load,
        );
        assert!(
            matches!(browser.screen(), Some(Screen::Outline(page)) if page.tree.selected == 1 && page.scroll == 5)
        );
        let mut list = files(None);
        list.messages
            .push("Watcher reported an unreadable directory".into());
        browser.apply(BrowserUpdate::Files(list), &document);
        assert_eq!(
            browser.messages().collect::<Vec<_>>(),
            [
                "invalid selector",
                "Watcher reported an unreadable directory"
            ]
        );
    }

    #[test]
    fn tab_preserves_the_node_and_enter_opens_the_file_root() {
        let mut browser = reading_passage();
        browser.key(KeyCode::Tab, 5, false, &document);
        assert!(
            matches!(browser.screen(), Some(Screen::Outline(page)) if page.tree.selected == 1 && page.scroll == 5)
        );
        browser.key(KeyCode::Enter, 5, false, &document);
        assert!(
            matches!(browser.screen(), Some(Screen::Outline(page)) if page.tree.selected == 0 && page.scroll == 0)
        );
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
        let (first_rows, first) = snapshot(&browser, 30, 7);
        assert!(first_rows.iter().any(|row| row == "> notes.md"));
        assert!(first_rows.iter().any(|row| row == &"a".repeat(30)));
        assert!(
            first_rows
                .iter()
                .any(|row| row.contains("PgUp/PgDn messages"))
        );
        assert!(first.more);
        browser.key(KeyCode::PageDown, first.page_height, first.more, &document);
        let (last_rows, last) = snapshot(&browser, 30, 7);
        assert!(!last.more);
        assert!(last_rows.iter().any(|row| row == "second warning"));
        assert!(last_rows.iter().any(|row| row == "last warning"));
        browser.key(KeyCode::PageUp, last.page_height, last.more, &document);
        assert_eq!(snapshot(&browser, 30, 7).0, first_rows);

        browser.key(KeyCode::PageDown, first.page_height, first.more, &document);
        let mut changed = files(None);
        changed.messages.push("new warning".into());
        browser.refresh(changed, &document);
        assert!(
            snapshot(&browser, 30, 7)
                .0
                .iter()
                .any(|row| row == "new warning")
        );
    }

    #[test]
    fn file_error_pages_survive_refresh_until_the_message_changes() {
        let mut browser = reading_passage();
        let message = format!("{}\nline 9, column 7", "x".repeat(44));
        browser.refresh(files(Some(&message)), &document);
        let (first_rows, first) = snapshot(&browser, 22, 5);
        assert!(first_rows.iter().any(|row| row == &"x".repeat(22)));
        assert!(first_rows.iter().any(|row| row.contains("PgUp/PgDn error")));
        assert!(first.more);

        // The number of page presses depends on how many help rows fit on this terminal.
        let mut pages = 0;
        let mut paging = first;
        while paging.more && pages < message.len() {
            assert!(paging.page_height > 0);
            browser.key(
                KeyCode::PageDown,
                paging.page_height,
                paging.more,
                &document,
            );
            paging = snapshot(&browser, 22, 5).1;
            pages += 1;
        }
        let (last_rows, last) = snapshot(&browser, 22, 5);
        assert!(last_rows.iter().any(|row| row == "line 9, column 7"));
        assert!(!last.more);
        browser.refresh(files(Some(&message)), &document);
        assert_eq!(snapshot(&browser, 22, 5).0, last_rows);
        for _ in 0..pages {
            browser.key(KeyCode::PageUp, last.page_height, false, &document);
        }
        assert_eq!(snapshot(&browser, 22, 5).0, first_rows);

        browser.key(KeyCode::PageDown, last.page_height, true, &document);
        browser.refresh(files(Some("new error")), &document);
        assert!(
            snapshot(&browser, 22, 5)
                .0
                .iter()
                .any(|row| row == "new error")
        );
    }

    #[test]
    fn narrow_outline_shows_navigation_and_exit_keys() {
        let browser = reading_passage();
        let rows = snapshot(&browser, 24, 12).0;
        for hint in [
            "Arrows navigate",
            "[ ] siblings",
            "PgUp/PgDn source",
            "Esc close",
            "q quit",
        ] {
            assert!(
                rows.iter().any(|row| row.contains(hint)),
                "{hint:?} missing from {rows:?}"
            );
        }
        assert!(rows.iter().any(|row| row.contains("passage")));
        assert!(rows.iter().any(|row| row.trim_end().ends_with("line 5")));
        assert_eq!(snapshot(&browser, 1, 1).0, ["q"]);
    }

    #[test]
    fn unrelated_refresh_preserves_the_visible_passage() {
        let mut browser = reading_passage();
        let before = snapshot(&browser, 80, 12).0;
        assert!(before.iter().any(|row| row.trim_end().ends_with("line 5")));
        browser.refresh(files(None), &document);
        assert_eq!(snapshot(&browser, 80, 12).0, before);
    }

    #[test]
    fn changing_the_selected_passage_resets_its_scroll() {
        let mut browser = reading_passage();
        browser.refresh(files(None), &|path| {
            let mut changed = document(path)?;
            changed.bytes[0] = b'L';
            Ok(changed)
        });
        let rows = snapshot(&browser, 80, 12).0;
        assert!(rows.iter().any(|row| row.trim_end().ends_with("Line 0")));
        assert!(
            rows.iter()
                .any(|row| row.contains('>') && row.contains("passage"))
        );
    }

    #[test]
    fn repair_restores_selection_without_exposing_stale_text() {
        let mut browser = reading_passage();
        let before = snapshot(&browser, 80, 12).0;
        for _ in 0..2 {
            browser.refresh(files(Some("syntax error")), &document);
            let error = snapshot(&browser, 80, 12).0;
            assert!(
                error
                    .iter()
                    .any(|row| row.trim_end().ends_with("syntax error"))
            );
            assert!(!error.iter().any(|row| row.contains("line 5")));
        }
        browser.refresh(files(None), &document);
        assert_eq!(snapshot(&browser, 80, 12).0, before);
    }
}
