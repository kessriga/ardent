//! Keyboard navigation and terminal display for the outline browser.

use std::io;

use ardent_core::outline::Outline;
use crossterm::event::KeyCode;

mod render;
mod terminal;
mod tree;

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

struct Browser {
    list: FileList,
    selected: usize,
    page: Option<Result<Page, String>>,
}

impl Browser {
    fn open(&mut self, load: &impl Fn(&str) -> io::Result<Document>) {
        let Some((path, error)) = self.list.files.get(self.selected) else {
            return;
        };
        let result = (|| {
            if let Some(error) = error {
                return Err(io::Error::other(error.clone()));
            }
            let document = load(path)?;
            let mut tree = Tree::new(&document.outline)?;
            if let Some(Ok(previous)) = &self.page {
                tree.restore(
                    &document.outline,
                    &previous.tree,
                    &previous.document.outline,
                );
            }
            Ok(Page {
                tree,
                document,
                scroll: 0,
            })
        })();
        self.page = Some(result.map_err(|err| err.to_string()));
    }

    fn refresh(&mut self, list: FileList, load: &impl Fn(&str) -> io::Result<Document>) {
        let current = self.list.files.get(self.selected).map(|(path, _)| path);
        let index = current.and_then(|path| list.files.binary_search_by(|(p, _)| p.cmp(path)).ok());
        self.list = list;
        self.selected =
            index.unwrap_or_else(|| self.selected.min(self.list.files.len().saturating_sub(1)));
        if index.is_none() {
            self.page = None;
        } else if self.page.is_some() {
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
        match (&mut self.page, key) {
            (_, KeyCode::Esc) => self.page = None,
            (None, KeyCode::Enter) => self.open(load),
            (None, KeyCode::Up) => self.selected = self.selected.saturating_sub(1),
            (None, KeyCode::Down) => {
                self.selected = (self.selected + 1).min(self.list.files.len().saturating_sub(1))
            }
            (Some(Ok(page)), KeyCode::PageUp) => page.scroll = page.scroll.saturating_sub(height),
            (Some(Ok(page)), KeyCode::PageDown) if more => page.scroll += height,
            (Some(Ok(page)), key) => {
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
/// The event loop calls `refresh` on the input thread. Return `Ok(None)` promptly while idle and
/// `Ok(Some(list))` with the complete replacement [`FileList`] after a sync.
///
/// # Errors
///
/// Returns an error if the terminal cannot be used or `refresh` fails.
pub fn run(
    list: FileList,
    load: impl Fn(&str) -> io::Result<Document>,
    refresh: impl FnMut() -> io::Result<Option<FileList>>,
) -> io::Result<()> {
    let browser = Browser {
        list,
        selected: 0,
        page: None,
    };
    terminal::run(browser, load, refresh)
}
