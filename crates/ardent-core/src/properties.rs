//! Properties supplied for the current revision of an outline.

use std::collections::BTreeMap;

/// A named display value. Its provider chooses whether it may appear beneath a node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub inline: bool,
}

/// Properties in display order, keyed by node index in the outline (including the file root).
pub type NodeProperties = BTreeMap<usize, Vec<Property>>;
