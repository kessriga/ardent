use std::ops::Range;

use ardent_core::outline::ScannedNode;

#[path = "../../../crates/ardent-format/src/engine.rs"]
mod engine;
#[path = "../../../crates/ardent-format/src/rust.rs"]
mod rust;

wit_bindgen::generate!();
struct Component;
export!(Component);

impl Guest for Component {
    fn describe() -> Metadata {
        Metadata {
            name: rust::NAME.into(),
            version: "0.1.0".into(),
            files: vec!["*.rs".into()],
            prefixes: vec![],
        }
    }

    fn scan(source: Vec<u8>) -> Result<Vec<Node>, ScanError> {
        thread_local! {
            static PLUGIN: Result<engine::FormatPlugin, String> =
                rust::plugin().map_err(|error| error.to_string());
        }
        let roots = PLUGIN.with(|plugin| {
            let plugin = plugin
                .as_ref()
                .map_err(|message| ScanError::Failure(message.clone()))?;
            plugin.scan(&source).map_err(|error| match error {
                engine::ScanError::Syntax { line, column } => ScanError::Syntax(Position {
                    line: line as u32,
                    column: column as u32,
                }),
                other => ScanError::Failure(other.to_string()),
            })
        })?;
        let mut nodes = Vec::new();
        flatten(roots, None, &mut nodes);
        Ok(nodes)
    }
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
