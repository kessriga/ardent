//! Syntax categories attached to byte ranges of a source revision.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Keyword,
    StringLiteral,
    Comment,
    Heading,
    Code,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub range: Range<usize>,
    pub category: Category,
}
