use std::ops::Range;

use ardent_core::outline::ScannedNode;

use super::RuntimeError;
use super::scanner::{ByteRange, Node};

// Ardent core builds paths recursively. Bound the depth before giving it a guest-supplied tree.
const MAX_DEPTH: usize = 256;
const PATH_BUDGET: usize = 64 << 20;
const FINGERPRINT_BUDGET: usize = 256 << 20;

struct OpenNode {
    index: u32,
    node: ScannedNode,
    path_bound: usize,
}

pub(super) fn tree(
    nodes: Vec<Node>,
    source_path: &str,
    source_len: usize,
) -> Result<Vec<ScannedNode>, RuntimeError> {
    let mut roots = Vec::new();
    let mut open: Vec<OpenNode> = Vec::new();
    let root_bound = source_path.len().saturating_mul(2);
    let mut path_bytes = PATH_BUDGET.saturating_sub(root_bound);
    let mut fingerprint_work = FINGERPRINT_BUDGET.saturating_sub(source_len);
    for (index, node) in nodes.into_iter().enumerate() {
        while open.last().map(|open| open.index) != node.parent {
            if !close_node(&mut open, &mut roots) {
                return Err(RuntimeError::Invalid("parent is not an open ancestor"));
            }
        }
        if open.len() == MAX_DEPTH {
            return Err(RuntimeError::Invalid("outline exceeds the nesting limit"));
        }
        let (parent_span, siblings, parent_bound) = match open.last() {
            Some(parent) => (
                parent.node.range.clone(),
                &parent.node.children,
                parent.path_bound,
            ),
            None => (0..source_len, &roots, root_bound),
        };
        // A byte may need escaping. A separator plus the longest u32 duplicate suffix takes 12 bytes.
        let path_bound = parent_bound
            .saturating_add(node.label.len().saturating_mul(2))
            .saturating_add(12);
        path_bytes = path_bytes
            .checked_sub(path_bound)
            .ok_or(RuntimeError::Invalid(
                "node paths exceed the format host's size limit",
            ))?;
        let node = checked_node(node, &parent_span, siblings)?;
        // Core checks each byte against the label ranges when computing a fingerprint.
        let work = node.range.len().saturating_mul(1 + node.label_ranges.len());
        fingerprint_work = fingerprint_work
            .checked_sub(work)
            .ok_or(RuntimeError::Invalid(
                "fingerprints exceed the format host's work limit",
            ))?;
        open.push(OpenNode {
            index: index as u32,
            path_bound,
            node,
        });
    }
    while close_node(&mut open, &mut roots) {}
    Ok(roots)
}

fn checked_node(
    node: Node,
    parent_span: &Range<usize>,
    siblings: &[ScannedNode],
) -> Result<ScannedNode, RuntimeError> {
    if node.kind.is_empty() || node.label.is_empty() {
        return Err(RuntimeError::Invalid("empty node kind or label"));
    }
    let range = checked_range(node.span, parent_span)?;
    if siblings
        .last()
        .is_some_and(|sibling| sibling.range.end > range.start)
    {
        return Err(RuntimeError::Invalid(
            "siblings overlap or are out of document order",
        ));
    }
    let label_ranges = node
        .label_ranges
        .into_iter()
        .map(|span| checked_range(span, &range))
        .collect::<Result<_, _>>()?;
    Ok(ScannedNode {
        kind: node.kind,
        label: node.label,
        range,
        label_ranges,
        children: Vec::new(),
    })
}

fn checked_range(span: ByteRange, container: &Range<usize>) -> Result<Range<usize>, RuntimeError> {
    let range = span.start as usize..span.end as usize;
    if range.start < container.start || range.end > container.end || range.end < range.start {
        return Err(RuntimeError::Invalid(
            "byte range lies outside its container or is reversed",
        ));
    }
    Ok(range)
}

fn close_node(open: &mut Vec<OpenNode>, roots: &mut Vec<ScannedNode>) -> bool {
    let Some(closed) = open.pop() else {
        return false;
    };
    match open.last_mut() {
        Some(parent) => parent.node.children.push(closed.node),
        None => roots.push(closed.node),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(parent: Option<u32>, start: u32, end: u32) -> Node {
        Node {
            parent,
            kind: "test".into(),
            label: "n".into(),
            span: ByteRange { start, end },
            label_ranges: vec![],
        }
    }

    #[test]
    fn equal_parent_and_child_ranges_are_valid() -> Result<(), RuntimeError> {
        let roots = tree(
            vec![node(None, 0, 4), node(Some(0), 0, 4), node(None, 4, 8)],
            "a",
            8,
        )?;
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].children.len(), 1);
        Ok(())
    }

    #[test]
    fn bounds_the_sum_of_paths_under_repeated_ancestors() {
        let mut parent = node(None, 0, 1);
        parent.label = "a".repeat(32_768);
        let mut nodes = vec![parent];
        nodes.extend((0..1024).map(|_| node(Some(0), 0, 0)));
        assert!(matches!(
            tree(nodes, "a", 1),
            Err(RuntimeError::Invalid(
                "node paths exceed the format host's size limit"
            ))
        ));
    }

    #[test]
    fn accounts_for_bytes_and_label_ranges_in_the_fingerprint_budget() {
        let mut leaf = node(None, 0, 1 << 20);
        leaf.label_ranges = (0..256)
            .map(|i| ByteRange {
                start: i,
                end: i + 1,
            })
            .collect();
        assert!(matches!(
            tree(vec![leaf], "file", 1 << 20),
            Err(RuntimeError::Invalid(
                "fingerprints exceed the format host's work limit"
            ))
        ));
    }

    #[test]
    fn rejects_invalid_ranges_order_and_parent_indices() {
        let mut bad_label = node(None, 1, 4);
        bad_label.label_ranges.push(ByteRange { start: 0, end: 1 });
        for invalid in [
            vec![bad_label],
            vec![node(None, 1, 0)],
            vec![node(None, 0, 9)],
            vec![node(Some(0), 0, 4)],
            vec![node(None, 0, 4), node(Some(0), 2, 6)],
            vec![node(None, 0, 4), node(None, 2, 6)],
            vec![node(None, 4, 8), node(None, 0, 4)],
            vec![node(None, 0, 4), node(None, 4, 8), node(Some(0), 1, 2)],
        ] {
            assert!(tree(invalid, "a", 8).is_err());
        }
    }
}
