//! Keyboard navigation over the nodes of one outline in document order.

use std::collections::HashSet;
use std::io;

use ardent_core::outline::Outline;
use crossterm::event::KeyCode;

struct Row {
    label: String,
    depth: usize,
    parent: Option<usize>,
    /// Exclusive end of the subtree in document order. A closed branch skips straight to this index.
    end: usize,
    expanded: bool,
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
        // Node paths give the second item a suffix and leave the first item unnumbered (decision 1).
        let first_of_many: HashSet<&str> = nodes
            .iter()
            .filter(|node| node.kind == "item")
            .filter_map(|node| node.path.strip_suffix("[2]"))
            .collect();
        let mut rows: Vec<Row> = Vec::with_capacity(nodes.len());
        let mut ancestors: Vec<usize> = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let (parent_path, mut label) = split_path(&node.path);
            if node.kind == "item" && label == "item" && first_of_many.contains(node.path.as_str())
            {
                label.push_str("[1]");
            }
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
                label,
                parent,
                depth: ancestors.len(),
                end: nodes.len(),
                expanded: index == 0,
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

    pub(crate) fn label(&self, index: usize) -> (usize, char, &str) {
        let row = &self.rows[index];
        let marker = if row.end == index + 1 {
            ' '
        } else if row.expanded {
            '-'
        } else {
            '+'
        };
        (row.depth, marker, &row.label)
    }

    pub(crate) fn navigate(&mut self, key: KeyCode) {
        let i = self.selected;
        let row = &self.rows[i];
        let branch = row.end > i + 1;
        let next = match key {
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
        let expanded: HashSet<_> = old
            .rows
            .iter()
            .zip(&previous.nodes)
            .filter(|(row, _)| row.expanded)
            .map(|(_, node)| &node.path)
            .collect();
        let selected = &previous.nodes[old.selected].path;
        for (index, (row, node)) in self.rows.iter_mut().zip(&outline.nodes).enumerate() {
            row.expanded = expanded.contains(&node.path);
            if &node.path == selected {
                self.selected = index;
            }
        }
    }
}

/// Keep the parent path escaped for lookup and decode the last label for display. Slashes in the source path and
/// escaped slashes in a label are literal. An unescaped slash after the first unescaped `#` separates labels.
fn split_path(path: &str) -> (Option<&str>, String) {
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
    let label = delimiter.map_or(path, |i| &path[i + 1..]);
    let mut chars = label.chars();
    let decoded = std::iter::from_fn(|| match chars.next()? {
        '\\' => chars.next(),
        c => Some(c),
    })
    .collect();
    (delimiter.map(|i| &path[..i]), decoded)
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
            r"docs/C\#.md#a\/b/x",
            r"docs/C\#.md#a\/b/x[2]",
            r"docs/C\#.md#last",
        ];
        let outline = Outline {
            nodes: names
                .iter()
                .map(|s| Node {
                    path: s.to_string(),
                    kind: "test".into(),
                    range: 0..5,
                    fingerprint: String::new(),
                })
                .collect(),
        };
        let mut tree = Tree::new(&outline)?;
        assert_eq!(tree.visible().collect::<Vec<_>>(), [0, 1, 4]);
        assert_eq!(tree.label(1).2, "a/b");
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
