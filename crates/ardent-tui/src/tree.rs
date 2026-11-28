//! Keyboard navigation over the nodes of one outline in document order.

use std::collections::HashMap;
use std::io;

use ardent_core::outline::Outline;
use crossterm::event::KeyCode;

struct Row {
    depth: usize,
    parent: Option<usize>,
    /// Exclusive end of the subtree in document order. A closed branch skips straight to this index.
    end: usize,
    expanded: bool,
    properties_open: bool,
}

pub(crate) struct Tree {
    rows: Vec<Row>,
    pub(crate) selected: usize,
}

impl Tree {
    /// Index the parent links in node paths. A parent and child can cover the same source range.
    pub(crate) fn new(outline: &Outline) -> io::Result<Self> {
        let nodes = &outline.nodes;
        let invalid =
            || io::Error::new(io::ErrorKind::InvalidData, "invalid node path in structure");
        if nodes.is_empty() {
            return Err(invalid());
        }
        let mut rows: Vec<Row> = Vec::with_capacity(nodes.len());
        let mut ancestors: Vec<usize> = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let parent_path = parent_path(&node.path);
            while let Some(&ancestor) = ancestors.last() {
                if Some(nodes[ancestor].path.as_str()) == parent_path {
                    break;
                }
                rows[ancestor].end = index;
                ancestors.pop();
            }
            let parent = ancestors.last().copied();
            if (index == 0) != parent.is_none() || parent.is_some() != parent_path.is_some() {
                return Err(invalid());
            }
            rows.push(Row {
                parent,
                depth: ancestors.len(),
                end: nodes.len(),
                expanded: index == 0,
                properties_open: false,
            });
            ancestors.push(index);
        }
        Ok(Self { rows, selected: 0 })
    }

    pub(crate) fn visible(&self) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(Some(0), |&i| {
            let row = &self.rows[i];
            let next = if row.expanded { i + 1 } else { row.end };
            (next < self.rows.len()).then_some(next)
        })
    }

    pub(crate) fn properties_open(&self, index: usize) -> bool {
        self.rows[index].properties_open
    }

    pub(crate) fn row_prefix(&self, index: usize) -> (usize, char) {
        let row = &self.rows[index];
        let marker = if row.end == index + 1 {
            ' '
        } else if row.expanded {
            '-'
        } else {
            '+'
        };
        (row.depth, marker)
    }

    pub(crate) fn navigate(&mut self, key: KeyCode) {
        let i = self.selected;
        let row = &self.rows[i];
        let branch = row.end > i + 1;
        let next = match key {
            KeyCode::Tab => {
                self.rows[i].properties_open = !row.properties_open;
                None
            }
            KeyCode::Right if branch && !row.expanded => {
                self.rows[i].expanded = true;
                None
            }
            KeyCode::Right if branch => Some(i + 1),
            KeyCode::Left if branch && row.expanded => {
                self.rows[i].expanded = false;
                None
            }
            KeyCode::Left => row.parent,
            KeyCode::Down => self.visible().find(|&j| j > i),
            KeyCode::Up => self.visible().take_while(|&j| j < i).last(),
            KeyCode::Char(']') => self
                .rows
                .get(row.end)
                .filter(|s| s.parent == row.parent)
                .map(|_| row.end),
            KeyCode::Char('[') => (0..i).rev().find(|&j| self.rows[j].parent == row.parent),
            _ => None,
        };
        if let Some(next) = next {
            self.selected = next;
        }
    }

    /// Carry expansion and selection across a sync by node path. Row indices belong to one outline.
    pub(crate) fn restore(&mut self, outline: &Outline, old: &Tree, previous: &Outline) {
        let open: HashMap<_, _> = old
            .rows
            .iter()
            .zip(&previous.nodes)
            .filter(|(row, _)| row.expanded || row.properties_open)
            .map(|(row, node)| (&node.path, row))
            .collect();
        let selected = &previous.nodes[old.selected].path;
        for (index, (row, node)) in self.rows.iter_mut().zip(&outline.nodes).enumerate() {
            let previous = open.get(&node.path);
            row.expanded = previous.is_some_and(|row| row.expanded);
            row.properties_open = previous.is_some_and(|row| row.properties_open);
            if &node.path == selected {
                self.selected = index;
            }
        }
    }
}

/// Find the parent without treating escaped separators or source directories as ancestors.
fn parent_path(path: &str) -> Option<&str> {
    let mut in_outline = false;
    let mut delimiter = None;
    let mut chars = path.char_indices();
    while let Some((i, ch)) = chars.next() {
        match ch {
            '\\' => {
                chars.next();
            }
            '#' => {
                in_outline = true;
                delimiter = Some(i);
            }
            '/' if in_outline => delimiter = Some(i),
            _ => {}
        }
    }
    delimiter.map(|i| &path[..i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ardent_core::outline::Node;

    #[test]
    fn nested_navigation_uses_paths_even_for_equal_ranges() -> io::Result<()> {
        use KeyCode::*;
        let names = [
            r"docs/C\#.md",
            r"docs/C\#.md#a\/b",
            r"docs/C\#.md#a\/b/x.1",
            r"docs/C\#.md#a\/b/x.2",
            r"docs/C\#.md#last",
        ];
        let outline = Outline {
            nodes: names
                .iter()
                .map(|s| Node {
                    path: s.to_string(),
                    label: String::new(),
                    kind: "test".into(),
                    range: 0..5,
                    fingerprint: String::new(),
                })
                .collect(),
        };
        let mut tree = Tree::new(&outline)?;
        assert_eq!(tree.visible().collect::<Vec<_>>(), [0, 1, 4]);
        for (key, expected) in [
            (Up, 0),
            (Right, 1),
            (Right, 1),
            (Right, 2),
            (Char(']'), 3),
            (Char(']'), 3),
            (Left, 1),
            (Char(']'), 4),
            (Char('['), 1),
            (Left, 1),
            (Down, 4),
            (Down, 4),
            (Up, 1),
            (Left, 0),
        ] {
            tree.navigate(key);
            assert_eq!(tree.selected, expected, "{key:?}");
        }
        Ok(())
    }
}
