use ardent_format_guest::properties::{self, Property};
use tree_sitter::Node;

pub(super) fn read(kind: &str, node: Node<'_>, _source: &[u8]) -> Vec<Property> {
    if kind != "item" {
        return Vec::new();
    }
    let mut cursor = node.walk();
    let checked = node
        .named_children(&mut cursor)
        .find_map(|child| match child.kind() {
            "task_list_marker_checked" => Some("true"),
            "task_list_marker_unchecked" => Some("false"),
            _ => None,
        });
    checked
        .into_iter()
        .map(|value| properties::inline("checked", value))
        .collect()
}
