//! The outline engine: runs the outline query of a format plugin on a tree-sitter parse tree and returns the nodes it
//! finds, nested by containment and by level (spec `outline`, decision-3, decision-5).

use std::collections::HashSet;
use std::fmt;
use std::ops::Range;

use ardent_core::outline::ScannedNode;
use tree_sitter::{
    Language, LanguageError, Parser, Query, QueryCursor, QueryError, StreamingIterator,
};

const NODE_CAPTURE: &str = "node";
const LABEL_CAPTURE: &str = "label";
const KIND_PROPERTY: &str = "kind";
const LEVEL_PROPERTY: &str = "level";

/// A format plugin: a tree-sitter grammar and an outline query, with the name, version, default file patterns, and
/// format hash that stamps and the configuration use (decision-3).
pub struct FormatPlugin {
    pub name: String,
    pub version: String,
    /// Gitignore-style patterns of the files that the plugin parses when `config.yaml` gives none.
    pub default_files: Vec<String>,
    /// The `format_hash` of stamps (spec `ardent-schemas`).
    pub format_hash: String,
    language: Language,
    query: CompiledQuery,
}

struct CompiledQuery {
    query: Query,
    node_capture: u32,
    label_capture: Option<u32>,
    /// The `kind` and `level` properties of each pattern, by pattern index.
    patterns: Vec<PatternProperties>,
}

struct PatternProperties {
    kind: Option<String>,
    level: Option<u32>,
}

/// Why a format plugin cannot be loaded.
#[derive(Debug)]
pub enum PluginError {
    Query(QueryError),
    /// The query has no `@node` capture, so it can find no node.
    MissingNodeCapture,
    /// A pattern sets `level` to a value that is not a positive integer.
    InvalidLevel {
        pattern: usize,
        value: String,
    },
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Query(err) => write!(f, "invalid outline query: {err}"),
            Self::MissingNodeCapture => {
                write!(f, "the outline query has no @{NODE_CAPTURE} capture")
            }
            Self::InvalidLevel { pattern, value } => write!(
                f,
                "pattern {} of the outline query sets level to {value:?}, which is not a positive integer",
                pattern + 1
            ),
        }
    }
}

impl std::error::Error for PluginError {}

/// Why a format plugin could not scan a file.
#[derive(Debug)]
pub enum ScanError {
    /// The grammar does not fit the tree-sitter version of Ardent.
    Language(LanguageError),
    /// tree-sitter returned no tree.
    Parse,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Language(err) => write!(f, "cannot load the grammar: {err}"),
            Self::Parse => f.write_str("the parser returned no syntax tree"),
        }
    }
}

impl std::error::Error for ScanError {}

/// A node that a match found, before nesting.
struct Candidate {
    kind: String,
    label: String,
    range: Range<usize>,
    label_ranges: Vec<Range<usize>>,
    level: Option<u32>,
    children: Vec<Candidate>,
}

impl FormatPlugin {
    pub fn new(
        name: &str,
        version: &str,
        default_files: &[&str],
        language: Language,
        query_source: &str,
        format_hash: String,
    ) -> Result<Self, PluginError> {
        Ok(Self {
            name: name.to_string(),
            version: version.to_string(),
            default_files: default_files.iter().map(|p| p.to_string()).collect(),
            format_hash,
            query: CompiledQuery::new(&language, query_source)?,
            language,
        })
    }

    /// Parses `source` and returns the top-level nodes of its outline, each with its descendants, in document order.
    pub fn scan(&self, source: &[u8]) -> Result<Vec<ScannedNode>, ScanError> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.language)
            .map_err(ScanError::Language)?;
        let tree = parser.parse(source, None).ok_or(ScanError::Parse)?;
        let candidates = self.query.candidates(tree.root_node(), source);
        let nested = nest_by_containment(candidates);
        Ok(nest_by_level(nested, source.len())
            .into_iter()
            .map(into_scanned)
            .collect())
    }
}

impl CompiledQuery {
    fn new(language: &Language, source: &str) -> Result<Self, PluginError> {
        let query = Query::new(language, source).map_err(PluginError::Query)?;
        let node_capture = query
            .capture_index_for_name(NODE_CAPTURE)
            .ok_or(PluginError::MissingNodeCapture)?;
        let label_capture = query.capture_index_for_name(LABEL_CAPTURE);
        let patterns = (0..query.pattern_count())
            .map(|pattern| PatternProperties::new(&query, pattern))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            query,
            node_capture,
            label_capture,
            patterns,
        })
    }

    /// One candidate for each syntax node that an `@node` capture matches, from the first pattern that matches it.
    fn candidates(&self, root: tree_sitter::Node<'_>, source: &[u8]) -> Vec<Candidate> {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.query, root, source);
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        while let Some(found) = matches.next() {
            let captures = found.captures();
            let Some(node) = captures.iter().find(|c| c.index == self.node_capture) else {
                continue;
            };
            if !seen.insert(node.node.id()) {
                continue;
            }
            let Some(properties) = self.patterns.get(found.pattern_index) else {
                continue;
            };
            let mut label_ranges: Vec<_> = captures
                .iter()
                .filter(|c| Some(c.index) == self.label_capture)
                .map(|c| c.node.byte_range())
                .collect();
            label_ranges.sort_by_key(|range| range.start);
            let kind = properties
                .kind
                .clone()
                .unwrap_or_else(|| node.node.kind().to_string());
            candidates.push(Candidate {
                label: label(source, &label_ranges).unwrap_or_else(|| kind.clone()),
                kind,
                range: node.node.byte_range(),
                label_ranges,
                level: properties.level,
                children: Vec::new(),
            });
        }
        candidates
    }
}

impl PatternProperties {
    fn new(query: &Query, pattern: usize) -> Result<Self, PluginError> {
        let mut properties = Self {
            kind: None,
            level: None,
        };
        for setting in query.property_settings(pattern) {
            let value = setting.value.as_deref().unwrap_or_default();
            match &*setting.key {
                KIND_PROPERTY => properties.kind = Some(value.to_string()),
                LEVEL_PROPERTY => {
                    let level = value.parse().ok().filter(|level| *level > 0);
                    properties.level = Some(level.ok_or_else(|| PluginError::InvalidLevel {
                        pattern,
                        value: value.to_string(),
                    })?);
                }
                _ => {}
            }
        }
        Ok(properties)
    }
}

/// Joins the text of the label captures with one space, collapses each run of ASCII whitespace to one space, and trims
/// the ends. `None` when the result is empty.
fn label(source: &[u8], label_ranges: &[Range<usize>]) -> Option<String> {
    let joined: Vec<u8> = label_ranges
        .iter()
        .filter_map(|range| source.get(range.clone()))
        .collect::<Vec<_>>()
        .join(&b' ');
    let text = String::from_utf8_lossy(&joined);
    let label = text
        .split(|c: char| c.is_ascii_whitespace() || c == '\u{0b}')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!label.is_empty()).then_some(label)
}

/// Makes each candidate a child of the nearest other candidate whose range contains it, and returns the candidates
/// that no other contains.
fn nest_by_containment(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    candidates.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(b.range.end.cmp(&a.range.end))
    });
    let mut top_level = Vec::new();
    let mut open: Vec<Candidate> = Vec::new();
    for candidate in candidates {
        while open
            .last()
            .is_some_and(|parent| !contains(&parent.range, &candidate.range))
        {
            close(&mut open, &mut top_level);
        }
        open.push(candidate);
    }
    while !open.is_empty() {
        close(&mut open, &mut top_level);
    }
    top_level
}

fn contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// Moves the last open candidate into the children of the one below it, or into `top_level`.
fn close(open: &mut Vec<Candidate>, top_level: &mut Vec<Candidate>) {
    if let Some(done) = open.pop() {
        match open.last_mut() {
            Some(parent) => parent.children.push(done),
            None => top_level.push(done),
        }
    }
}

/// Applies "Nesting by level" to a list of siblings whose parent ends at `parent_end`, after it applied the rule to the
/// children of each sibling.
fn nest_by_level(siblings: Vec<Candidate>, parent_end: usize) -> Vec<Candidate> {
    let mut result = Vec::new();
    let mut open: Vec<Candidate> = Vec::new();
    for mut sibling in siblings {
        sibling.children = nest_by_level(std::mem::take(&mut sibling.children), sibling.range.end);
        if let Some(level) = sibling.level {
            while open.last().is_some_and(|run| run.level >= Some(level)) {
                close_run(&mut open, &mut result, sibling.range.start);
            }
            open.push(sibling);
        } else if let Some(run) = open.last_mut() {
            run.children.push(sibling);
        } else {
            result.push(sibling);
        }
    }
    while !open.is_empty() {
        close_run(&mut open, &mut result, parent_end);
    }
    result
}

/// Ends the run of the last open node with a level at `end`, and moves the node into the run below it or into
/// `result`.
fn close_run(open: &mut Vec<Candidate>, result: &mut Vec<Candidate>, end: usize) {
    if let Some(run) = open.last_mut() {
        run.range.end = run.range.end.max(end);
    }
    close(open, result);
}

fn into_scanned(candidate: Candidate) -> ScannedNode {
    ScannedNode {
        kind: candidate.kind,
        label: candidate.label,
        range: candidate.range,
        label_ranges: candidate.label_ranges,
        children: candidate.children.into_iter().map(into_scanned).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markdown(query: &str) -> Result<FormatPlugin, PluginError> {
        FormatPlugin::new(
            "test",
            "0.0.0",
            &["*.md"],
            tree_sitter_md::LANGUAGE.into(),
            query,
            String::new(),
        )
    }

    /// Each node as `kind label start..end`, indented by depth.
    fn render(nodes: &[ScannedNode], depth: usize, out: &mut String) {
        for node in nodes {
            let indent = "  ".repeat(depth);
            out.push_str(&format!(
                "{indent}{} {} {}..{}\n",
                node.kind, node.label, node.range.start, node.range.end
            ));
            render(&node.children, depth + 1, out);
        }
    }

    fn outline(query: &str, source: &str) -> Result<String, Box<dyn std::error::Error>> {
        let nodes = markdown(query)?.scan(source.as_bytes())?;
        let mut out = String::new();
        render(&nodes, 0, &mut out);
        Ok(out)
    }

    const HEADINGS: &str = r#"
((atx_heading (atx_h2_marker) heading_content: (inline) @label) @node (#set! kind "heading") (#set! level "2"))
((atx_heading (atx_h3_marker) heading_content: (inline) @label) @node (#set! kind "heading") (#set! level "3"))
((section (paragraph) @node) (#set! kind "paragraph"))
"#;

    #[test]
    fn paragraph_pattern_gives_one_node() -> Result<(), Box<dyn std::error::Error>> {
        let query = r#"((paragraph) @node (#set! kind "paragraph"))"#;
        assert_eq!(
            outline(query, "Ardent derives an outline from each source file.\n")?,
            "paragraph paragraph 0..49\n"
        );
        Ok(())
    }

    #[test]
    fn kind_defaults_to_the_syntax_node_type() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            outline("(block_quote) @node", "> quoted\n")?,
            "block_quote block_quote 0..9\n"
        );
        Ok(())
    }

    #[test]
    fn one_node_for_a_syntax_node_that_two_patterns_match() -> Result<(), Box<dyn std::error::Error>>
    {
        let query =
            r#"((paragraph) @node (#set! kind "first")) ((paragraph) @node (#set! kind "second"))"#;
        assert_eq!(outline(query, "Text.\n")?, "first first 0..6\n");
        Ok(())
    }

    #[test]
    fn nests_by_containment() -> Result<(), Box<dyn std::error::Error>> {
        let query = r#"((list) @node (#set! kind "list")) ((list_item) @node (#set! kind "item"))"#;
        assert_eq!(
            outline(query, "- a\n- b\n  - c\n")?,
            "list list 0..14\n  item item 0..4\n  item item 4..14\n    list list 10..14\n      item item 10..14\n"
        );
        Ok(())
    }

    #[test]
    fn nests_by_level() -> Result<(), Box<dyn std::error::Error>> {
        let source = "## A\n\nOne.\n\n### B\n\nTwo.\n\n## C\n";
        assert_eq!(
            outline(HEADINGS, source)?,
            "heading A 0..25\n  paragraph paragraph 6..11\n  heading B 12..25\n    paragraph paragraph 19..24\nheading C 25..30\n"
        );
        Ok(())
    }

    #[test]
    fn a_skipped_level_still_nests() -> Result<(), Box<dyn std::error::Error>> {
        let query = r#"
((atx_heading (atx_h1_marker) heading_content: (inline) @label) @node (#set! level "1"))
((atx_heading (atx_h2_marker) heading_content: (inline) @label) @node (#set! level "2"))
((atx_heading (atx_h3_marker) heading_content: (inline) @label) @node (#set! level "3"))
"#;
        assert_eq!(
            outline(query, "# A\n### B\n## C\n")?,
            "atx_heading A 0..15\n  atx_heading B 4..10\n  atx_heading C 10..15\n"
        );
        Ok(())
    }

    #[test]
    fn joins_label_captures_and_collapses_whitespace() -> Result<(), Box<dyn std::error::Error>> {
        let query = r#"((setext_heading heading_content: (paragraph (inline) @label)) @node (#set! kind "heading"))"#;
        assert_eq!(
            outline(query, "Setext\n  title\n===\n")?,
            "heading Setext title 0..19\n"
        );
        let query = r#"((fenced_code_block (fenced_code_block_delimiter) @label (info_string) @label) @node)"#;
        assert_eq!(
            outline(query, "```sh\nls\n```\n")?,
            "fenced_code_block ``` sh 0..13\n"
        );
        Ok(())
    }

    #[test]
    fn label_without_text_is_the_kind() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(outline(HEADINGS, "Text.\n")?, "paragraph paragraph 0..6\n");
        Ok(())
    }

    #[test]
    fn rejects_an_invalid_level() {
        for value in ["two", "0", "-1", ""] {
            let query = format!(r#"((paragraph) @node (#set! level "{value}"))"#);
            assert!(
                matches!(
                    markdown(&query),
                    Err(PluginError::InvalidLevel { pattern: 0, .. })
                ),
                "{value:?}"
            );
        }
    }

    #[test]
    fn rejects_a_query_without_node_capture() {
        assert!(matches!(
            markdown("(paragraph) @text"),
            Err(PluginError::MissingNodeCapture)
        ));
    }
}
