//! Directory navigation for the project's tracked source files.

use std::collections::{HashMap, HashSet};

use crossterm::event::KeyCode;

#[derive(Clone)]
pub(super) struct Entry {
    /// Full project-relative path. Files with the same name in different directories stay distinct.
    pub(super) path: String,
    pub(super) depth: usize,
    pub(super) directory: bool,
}

/// The explorer cursor and collapsed directories belong to the sidebar, independently of the open file.
#[derive(Default)]
pub(super) struct Explorer {
    children: HashMap<String, Vec<Entry>>,
    collapsed: HashSet<String>,
    rows: Vec<Entry>,
    selected: usize,
}

impl Explorer {
    pub(super) fn new(files: &[(String, Option<String>)]) -> Self {
        let mut explorer = Self::default();
        explorer.refresh(files);
        explorer
    }

    pub(super) fn rows(&self) -> &[Entry] {
        &self.rows
    }

    pub(super) fn selected(&self) -> usize {
        self.selected
    }

    pub(super) fn selected_entry(&self) -> Option<&Entry> {
        self.rows.get(self.selected)
    }

    pub(super) fn is_expanded(&self, directory: &str) -> bool {
        !self.collapsed.contains(directory)
    }

    /// Preserve the selected path across a sync. If it disappeared, prefer a surviving ancestor.
    pub(super) fn refresh(&mut self, files: &[(String, Option<String>)]) {
        let previous = self.selected_entry().map(|entry| entry.path.clone());
        self.children.clear();
        for (path, _) in files {
            self.insert_file(path);
        }
        for siblings in self.children.values_mut() {
            siblings.sort_unstable_by(|a, b| {
                b.directory
                    .cmp(&a.directory)
                    .then_with(|| a.path.cmp(&b.path))
            });
        }
        self.collapsed
            .retain(|path| self.children.contains_key(path));
        self.rows = self.visible_children("");
        self.selected = previous
            .and_then(|path| {
                self.rows.iter().rposition(|entry| {
                    entry.path == path
                        || path
                            .strip_prefix(&entry.path)
                            .is_some_and(|rest| rest.starts_with('/'))
                })
            })
            .unwrap_or_else(|| self.selected.min(self.rows.len().saturating_sub(1)));
    }

    fn insert_file(&mut self, mut path: &str) {
        let mut depth = path.bytes().filter(|&byte| byte == b'/').count();
        let mut directory = false;
        loop {
            let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
            let entry = Entry {
                path: path.to_owned(),
                depth,
                directory,
            };
            // A known parent already has an entry in its own parent directory.
            if let Some(siblings) = self.children.get_mut(parent) {
                siblings.push(entry);
                break;
            }
            self.children.insert(parent.to_owned(), vec![entry]);
            if parent.is_empty() {
                break;
            }
            path = parent;
            depth -= 1;
            directory = true;
        }
    }

    fn visible_children(&self, directory: &str) -> Vec<Entry> {
        let mut rows = Vec::new();
        let mut pending: Vec<_> = self
            .children
            .get(directory)
            .into_iter()
            .flatten()
            .rev()
            .collect();
        while let Some(entry) = pending.pop() {
            rows.push(entry.clone());
            if entry.directory
                && self.is_expanded(&entry.path)
                && let Some(children) = self.children.get(&entry.path)
            {
                pending.extend(children.iter().rev());
            }
        }
        rows
    }

    pub(super) fn navigate(&mut self, key: KeyCode) {
        let Some(entry) = self.selected_entry() else {
            return;
        };
        let expanded = entry.directory && self.is_expanded(&entry.path);
        match key {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => self.selected = (self.selected + 1).min(self.rows.len() - 1),
            KeyCode::Left | KeyCode::Enter if expanded => {
                let depth = entry.depth;
                self.collapsed.insert(entry.path.clone());
                let start = self.selected + 1;
                let count = self.rows[start..]
                    .iter()
                    .take_while(|row| row.depth > depth)
                    .count();
                self.rows.drain(start..start + count);
            }
            KeyCode::Right | KeyCode::Enter if entry.directory && !expanded => {
                let children = self.visible_children(&entry.path);
                let path = entry.path.clone();
                self.collapsed.remove(&path);
                let start = self.selected + 1;
                self.rows.splice(start..start, children);
            }
            KeyCode::Right if expanded => self.selected += 1,
            KeyCode::Left => {
                if let Some(parent) = self.rows[..self.selected]
                    .iter()
                    .rposition(|row| row.depth < entry.depth)
                {
                    self.selected = parent;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use KeyCode::{Down, Enter, Left, Right, Up};

    fn files(paths: &[&str]) -> Vec<(String, Option<String>)> {
        paths
            .iter()
            .map(|path| ((*path).to_owned(), None))
            .collect()
    }

    fn paths(explorer: &Explorer) -> Vec<&str> {
        explorer
            .rows()
            .iter()
            .map(|entry| entry.path.as_str())
            .collect()
    }

    #[test]
    fn directories_sort_before_files_and_retain_nested_folds() {
        let mut explorer = Explorer::new(&files(&[
            "root.md",
            "docs/file.md",
            "docs/sub/file.md",
            "aa.md",
        ]));
        assert_eq!(
            paths(&explorer),
            [
                "docs",
                "docs/sub",
                "docs/sub/file.md",
                "docs/file.md",
                "aa.md",
                "root.md"
            ]
        );
        for (key, expected) in [
            (Right, "docs/sub"),
            (Enter, "docs/sub"),
            (Left, "docs"),
            (Left, "docs"),
            (Down, "aa.md"),
            (Up, "docs"),
            (Right, "docs"),
            (Right, "docs/sub"),
        ] {
            explorer.navigate(key);
            assert_eq!(
                explorer.selected_entry().map(|e| e.path.as_str()),
                Some(expected)
            );
        }
        assert_eq!(
            paths(&explorer),
            ["docs", "docs/sub", "docs/file.md", "aa.md", "root.md"]
        );
        explorer.navigate(Right);
        explorer.navigate(Right);
        assert_eq!(
            explorer.selected_entry().map(|e| e.path.as_str()),
            Some("docs/sub/file.md")
        );
    }

    #[test]
    fn sync_preserves_folds_and_selection_then_recovers_from_removal() {
        let mut explorer = Explorer::new(&files(&["src/α.md", "src/sub/code.rs", "top.md"]));
        explorer.navigate(Right);
        explorer.navigate(Enter);
        explorer.navigate(Down);
        explorer.refresh(&files(&[
            "aardvark/new.md",
            "src/α.md",
            "src/sub/new.rs",
            "top.md",
        ]));
        assert_eq!(
            explorer.selected_entry().map(|e| e.path.as_str()),
            Some("src/α.md")
        );
        assert!(!explorer.is_expanded("src/sub"));
        explorer.refresh(&files(&["aardvark/new.md", "src/sub/new.rs", "top.md"]));
        assert_eq!(
            explorer.selected_entry().map(|e| e.path.as_str()),
            Some("src")
        );
        explorer.refresh(&files(&["single.md"]));
        assert_eq!(
            explorer.selected_entry().map(|e| e.path.as_str()),
            Some("single.md")
        );
        explorer.refresh(&[]);
        explorer.navigate(Down);
        assert!(explorer.selected_entry().is_none());
    }
}
