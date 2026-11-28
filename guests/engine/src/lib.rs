//! Guest-side bindings for a format component backed by a tree-sitter query.

use std::ops::Range;

use ardent_core::outline::ScannedNode;

mod engine;
pub use engine::{PluginError, QueryEngine, ScanError as QueryScanError};

wit_bindgen::generate!({ path: "../../wit", world: "format-plugin", pub_export_macro: true });

pub mod highlighting {
    use tree_sitter::{Node, Tree};

    /// Visit nodes in document order. The callback returns whether to descend into the node.
    pub fn visit(tree: &Tree, mut descend: impl FnMut(Node<'_>) -> bool) {
        let mut cursor = tree.walk();
        loop {
            if descend(cursor.node()) && cursor.goto_first_child() {
                continue;
            }
            while !cursor.goto_next_sibling() {
                if !cursor.goto_parent() {
                    return;
                }
            }
        }
    }

    wit_bindgen::generate!({ path: "../../wit", world: "syntax-highlighting", pub_export_macro: true });
    pub use exports::ardent::format::highlighter::{Category, Guest, Span};
}

pub mod properties {
    wit_bindgen::generate!({ path: "../../wit", world: "source-properties", pub_export_macro: true });
    pub use exports::ardent::format::node_properties::{Group, Guest, Property};

    pub fn inline(name: &str, value: &str) -> Property {
        Property {
            name: name.to_owned(),
            value: value.to_owned(),
            inline: true,
        }
    }
}

pub mod query {
    wit_bindgen::generate!({ path: "../../wit", world: "query-setup", pub_export_macro: true });
    pub use exports::ardent::format::query_preparation::Guest;
}
pub use exports::ardent::format::scanner;
use scanner::{ByteRange, Metadata, Node, Position, ScanError};

pub fn metadata(name: &str, version: &str, files: &[&str]) -> Metadata {
    Metadata {
        name: name.to_owned(),
        version: version.to_owned(),
        files: files.iter().map(|pattern| (*pattern).to_owned()).collect(),
        prefixes: Vec::new(),
    }
}

pub fn scan(
    engine: Result<&QueryEngine, &PluginError>,
    source: &[u8],
) -> Result<Vec<Node>, ScanError> {
    let engine = engine.map_err(|error| ScanError::Failure(error.to_string()))?;
    let roots = engine.scan(source).map_err(|error| match error {
        QueryScanError::Syntax { line, column } => ScanError::Syntax(Position {
            line: line as u32,
            column: column as u32,
        }),
        other => ScanError::Failure(other.to_string()),
    })?;
    let mut nodes = Vec::new();
    flatten(roots, None, &mut nodes);
    Ok(nodes)
}

fn byte_range(range: Range<usize>) -> ByteRange {
    ByteRange {
        start: range.start as u32,
        end: range.end as u32,
    }
}

fn flatten(children: Vec<ScannedNode>, parent: Option<u32>, nodes: &mut Vec<Node>) {
    for child in children {
        let index = nodes.len() as u32;
        nodes.push(Node {
            parent,
            kind: child.kind,
            label: child.label,
            span: byte_range(child.range),
            label_ranges: child.label_ranges.into_iter().map(byte_range).collect(),
        });
        flatten(child.children, Some(index), nodes);
    }
}
