//! The outline of a source file: its nodes in document order, root first, each with its node path and fingerprint, as
//! decision-1 and the spec `outline` define them.

use std::collections::HashMap;
use std::ops::Range;

use serde::{Deserialize, Serialize};

/// Kind of the root node of every outline.
pub const FILE_KIND: &str = "file";

/// Characters that a backslash escapes in a label.
const LABEL_SPECIALS: &[char] = &['/', '#', '[', ']', '\\'];
/// Characters that a backslash escapes in a source path, where `/` separates directories.
const SOURCE_PATH_SPECIALS: &[char] = &['#', '[', ']', '\\'];

/// One node as a format plugin supplies it (decision-1). Ranges are byte ranges into the source file, and `children`
/// are in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedNode {
    pub kind: String,
    pub label: String,
    pub range: Range<usize>,
    pub label_ranges: Vec<Range<usize>>,
    pub children: Vec<ScannedNode>,
}

/// One node of an outline, with the keys of a node in a structure file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub path: String,
    pub kind: String,
    #[serde(with = "crate::yaml::range")]
    pub range: Range<usize>,
    pub fingerprint: String,
}

/// The nodes of one source file in document order, root first. The path of each node gives its parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Outline {
    pub nodes: Vec<Node>,
}

impl Outline {
    /// Whether the root node records the given source path, including its current spelling.
    pub fn matches_source_path(&self, source_path: &str) -> bool {
        self.nodes
            .first()
            .is_some_and(|root| root.path == escape(source_path, SOURCE_PATH_SPECIALS))
    }

    /// Builds the outline of the source file at `source_path` from the top-level nodes that a format plugin found in
    /// `source`. Adds the root node, and gives every node its node path and fingerprint.
    pub fn build(source_path: &str, source: &[u8], top_level: &[ScannedNode]) -> Self {
        let root_path = escape(source_path, SOURCE_PATH_SPECIALS);
        let root = Node {
            kind: FILE_KIND.to_string(),
            range: 0..source.len(),
            fingerprint: fingerprint(source, 0..source.len(), &[]),
            path: root_path.clone(),
        };
        let mut nodes = vec![root];
        push_children(source, &format!("{root_path}#"), top_level, &mut nodes);
        Self { nodes }
    }
}

/// Appends `children` and their descendants in document order. `prefix` is the parent path with its trailing `#` or
/// `/`.
fn push_children(source: &[u8], prefix: &str, children: &[ScannedNode], nodes: &mut Vec<Node>) {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for child in children {
        let count = seen.entry(child.label.as_str()).or_default();
        *count += 1;
        let suffix = if *count == 1 {
            String::new()
        } else {
            format!("[{count}]")
        };
        let path = format!("{prefix}{}{suffix}", escape(&child.label, LABEL_SPECIALS));
        nodes.push(Node {
            path: path.clone(),
            kind: child.kind.clone(),
            range: child.range.clone(),
            fingerprint: fingerprint(source, child.range.clone(), &child.label_ranges),
        });
        push_children(source, &format!("{path}/"), &child.children, nodes);
    }
}

fn escape(text: &str, specials: &[char]) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        if specials.contains(&c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// The fingerprint of the spec `ardent-schemas`: the BLAKE3 hash of the node's text without its label ranges, with each
/// run of ASCII whitespace replaced by one space and the ends trimmed, cut to 8 bytes as 16 hex digits. A range outside
/// `source` counts as empty text.
pub fn fingerprint(source: &[u8], range: Range<usize>, label_ranges: &[Range<usize>]) -> String {
    let text = source.get(range.clone()).unwrap_or_default();
    let mut normalized = Vec::with_capacity(text.len());
    let mut space_pending = false;
    for (offset, &byte) in text.iter().enumerate() {
        let position = range.start + offset;
        if label_ranges.iter().any(|label| label.contains(&position)) {
            continue;
        }
        if matches!(byte, 0x09..=0x0d | b' ') {
            space_pending = !normalized.is_empty();
        } else {
            if space_pending {
                normalized.push(b' ');
                space_pending = false;
            }
            normalized.push(byte);
        }
    }
    blake3::hash(&normalized)
        .to_hex()
        .chars()
        .take(16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Rust example of the spec `ardent-schemas`.
    const ENGINE_RS: &str = "pub struct Engine {
    files: Vec<String>,
}

impl Engine {
    pub fn run(&self) -> usize {
        self.files.len()
    }

    fn reset(&mut self) {
        self.files.clear();
    }
}
";

    fn leaf(kind: &str, label: &str, range: Range<usize>) -> ScannedNode {
        ScannedNode {
            kind: kind.to_string(),
            label: label.to_string(),
            range,
            label_ranges: Vec::new(),
            children: Vec::new(),
        }
    }

    fn paths(outline: &Outline) -> Vec<&str> {
        outline
            .nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect()
    }

    #[test]
    fn root_node_of_an_empty_file() {
        let outline = Outline::build("notes.md", b"", &[]);
        assert_eq!(
            outline.nodes,
            [Node {
                path: "notes.md".to_string(),
                kind: "file".to_string(),
                range: 0..0,
                fingerprint: fingerprint(b"", 0..0, &[]),
            }]
        );
    }

    #[test]
    fn nested_node_path() {
        let mut sync_rules = leaf("heading", "Sync rules", 0..1);
        sync_rules.children.push(leaf("heading", "Renames", 0..1));
        let outline = Outline::build("docs/spec.md", b"x", &[sync_rules]);
        assert_eq!(
            paths(&outline),
            [
                "docs/spec.md",
                "docs/spec.md#Sync rules",
                "docs/spec.md#Sync rules/Renames"
            ]
        );
    }

    #[test]
    fn escapes_labels_and_source_paths() {
        let outline = Outline::build(
            "docs/C#.md",
            b"x",
            &[leaf("heading", "Read/write [all] \\ #1", 0..1)],
        );
        assert_eq!(
            paths(&outline),
            [
                "docs/C\\#.md",
                "docs/C\\#.md#Read\\/write \\[all\\] \\\\ \\#1"
            ]
        );
    }

    #[test]
    fn suffixes_repeated_labels() {
        let mut sync_rules = leaf("heading", "Sync rules", 0..1);
        sync_rules.children = vec![
            leaf("paragraph", "paragraph", 0..1),
            leaf("list", "list", 0..1),
            leaf("paragraph", "paragraph", 0..1),
            leaf("paragraph", "paragraph", 0..1),
        ];
        let install = leaf("heading", "Install", 0..1);
        let outline = Outline::build("README.md", b"x", &[sync_rules, install.clone(), install]);
        assert_eq!(
            paths(&outline),
            [
                "README.md",
                "README.md#Sync rules",
                "README.md#Sync rules/paragraph",
                "README.md#Sync rules/list",
                "README.md#Sync rules/paragraph[2]",
                "README.md#Sync rules/paragraph[3]",
                "README.md#Install",
                "README.md#Install[2]",
            ]
        );
    }

    #[test]
    fn fingerprint_of_the_rust_example() {
        assert_eq!(ENGINE_RS.len(), 188);
        let run = fingerprint(ENGINE_RS.as_bytes(), 65..124, &[69..71, 72..75]);
        assert_eq!(run, "a33b27c3382d2785");
        assert_eq!(
            fingerprint(ENGINE_RS.as_bytes(), 0..188, &[]),
            "174add0298423e83"
        );
    }

    #[test]
    fn fingerprint_ignores_line_endings() {
        let crlf = ENGINE_RS.replace('\n', "\r\n");
        assert_eq!(
            fingerprint(crlf.as_bytes(), 0..crlf.len(), &[]),
            fingerprint(ENGINE_RS.as_bytes(), 0..ENGINE_RS.len(), &[])
        );
    }

    #[test]
    fn fingerprint_survives_a_rename() {
        let renamed = ENGINE_RS.replace("fn run", "fn start");
        assert_eq!(
            fingerprint(renamed.as_bytes(), 65..126, &[69..71, 72..77]),
            "a33b27c3382d2785"
        );
    }

    #[test]
    fn fingerprint_of_a_range_outside_the_source_is_that_of_empty_text() {
        assert_eq!(
            fingerprint(b"short", 3..99, &[]),
            fingerprint(b"", 0..0, &[])
        );
    }
}
