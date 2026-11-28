use ardent_format_guest::properties::{self, Property};
use tree_sitter::Node;

pub(super) fn read(kind: &str, node: Node<'_>, source: &[u8]) -> Vec<Property> {
    let mut result = Vec::new();
    let tuple_field = kind == "field" && node.kind() != "field_declaration";
    if let Some(value) = visibility(kind, node).and_then(|v| v.utf8_text(source).ok()) {
        result.push(properties::inline("visibility", value));
    }

    let declared_type = match kind {
        "return" => Some(node),
        "field" if tuple_field => Some(node),
        "parameter"
            if !matches!(
                node.kind(),
                "parameter" | "self_parameter" | "variadic_parameter"
            ) =>
        {
            Some(node)
        }
        "static" | "const" | "field" | "type" | "parameter" | "generic" => {
            node.child_by_field_name("type")
        }
        _ => None,
    };
    if let Some(value) = declared_type.and_then(|ty| ty.utf8_text(source).ok()) {
        result.push(properties::inline("type", value));
    }
    if kind == "generic"
        && let Some(bounds) = node.child_by_field_name("bounds")
        && let Ok(text) = bounds.utf8_text(source)
        && let Some(value) = text.strip_prefix(':')
    {
        result.push(properties::inline("bound", value.trim()));
    }
    result
}

fn visibility<'tree>(kind: &str, node: Node<'tree>) -> Option<Node<'tree>> {
    let owner = node.parent().and_then(|parent| parent.parent());
    match kind {
        "field" => {
            if owner.is_some_and(|owner| owner.kind() == "enum_variant") {
                return None;
            }
            if node.kind() != "field_declaration" {
                let mut previous = node.prev_sibling();
                while previous.is_some_and(|sibling| sibling.is_extra()) {
                    previous = previous.and_then(|sibling| sibling.prev_sibling());
                }
                return previous.filter(|sibling| sibling.kind() == "visibility_modifier");
            }
        }
        "fn" | "type" | "const" => {
            if owner.is_some_and(|owner| {
                owner.kind() == "trait_item"
                    || (owner.kind() == "impl_item" && owner.child_by_field_name("trait").is_some())
            }) {
                return None;
            }
        }
        "mod" | "struct" | "union" | "enum" | "trait" | "static" => {}
        _ => return None,
    }
    node.named_children(&mut node.walk())
        .find(|child| child.kind() == "visibility_modifier")
}
