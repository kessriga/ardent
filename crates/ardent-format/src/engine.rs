//! The outline engine: runs the outline query of a format plugin on a tree-sitter parse tree and returns the nodes it
//! finds, nested by containment and by level (spec `outline`, decision-3, decision-5, decision-6, decision-7).

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::iter;
use std::mem;
use std::ops::Range;

use ardent_core::outline::ScannedNode;
use blake3::Hasher;
use tree_sitter::{
    Language, LanguageError, Node, Parser, Query, QueryCursor, QueryError, QueryMatch,
    StreamingIterator,
};

const NODE_CAPTURE: &str = "node";
const LABEL_CAPTURE: &str = "label";
const ANNOTATION_CAPTURE: &str = "annotation";
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
    annotation_capture: Option<u32>,
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
    /// A pattern sets `kind` without a value or to the empty string, which gives no usable node path.
    EmptyKind {
        pattern: usize,
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
            Self::EmptyKind { pattern } => write!(
                f,
                "pattern {} of the outline query sets an empty kind",
                pattern + 1
            ),
        }
    }
}

impl Error for PluginError {}

/// Why a format plugin could not scan a file.
#[derive(Debug)]
pub enum ScanError {
    /// The grammar does not fit the tree-sitter version of Ardent.
    Language(LanguageError),
    /// tree-sitter returned no tree.
    Parse,
    /// The parse tree has an `ERROR` or `MISSING` node. Both numbers count from 1, and `column` counts characters.
    Syntax { line: usize, column: usize },
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Language(err) => write!(f, "cannot load the grammar: {err}"),
            Self::Parse => f.write_str("the parser returned no syntax tree"),
            Self::Syntax { line, column } => {
                write!(f, "syntax error at line {line}, column {column}")
            }
        }
    }
}

impl Error for ScanError {}

/// A node that a match found, before nesting.
struct Candidate {
    kind: String,
    label: String,
    range: Range<usize>,
    label_ranges: Vec<Range<usize>>,
    level: Option<u32>,
    /// The number of ancestors of the syntax node, which orders two candidates with the same range.
    depth: usize,
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
    /// A file with a syntax error gives no nodes (decision-7).
    pub fn scan(&self, source: &[u8]) -> Result<Vec<ScannedNode>, ScanError> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.language)
            .map_err(ScanError::Language)?;
        let tree = parser.parse(source, None).ok_or(ScanError::Parse)?;
        if tree.root_node().has_error() {
            return Err(syntax_error(tree.root_node(), source));
        }
        let candidates = self.query.candidates(tree.root_node(), source);
        let nested = nest_by_containment(candidates);
        Ok(nest_by_level(nested, source.len())
            .into_iter()
            .map(into_scanned)
            .collect())
    }
}

/// The format hash of a built-in plugin (spec `ardent-schemas`): the BLAKE3 hash of the query text, then the name and
/// version of the grammar crate.
pub(crate) fn builtin_format_hash(
    query: &str,
    grammar_crate: &str,
    grammar_version: &str,
) -> String {
    let mut hasher = Hasher::new();
    hasher.update(query.as_bytes());
    hasher.update(grammar_crate.as_bytes());
    hasher.update(grammar_version.as_bytes());
    hasher.finalize().to_hex().to_string()
}

impl CompiledQuery {
    fn new(language: &Language, source: &str) -> Result<Self, PluginError> {
        let query = Query::new(language, source).map_err(PluginError::Query)?;
        let node_capture = query
            .capture_index_for_name(NODE_CAPTURE)
            .ok_or(PluginError::MissingNodeCapture)?;
        let label_capture = query.capture_index_for_name(LABEL_CAPTURE);
        let annotation_capture = query.capture_index_for_name(ANNOTATION_CAPTURE);
        let patterns = (0..query.pattern_count())
            .map(|pattern| PatternProperties::new(&query, pattern))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            query,
            node_capture,
            label_capture,
            annotation_capture,
            patterns,
        })
    }

    /// One candidate for each syntax node that an `@node` capture matches, from the first pattern that matches it.
    /// Matches arrive in the order they finish, so a later pattern can match the same syntax node first.
    fn candidates(&self, root: Node<'_>, source: &[u8]) -> Vec<Candidate> {
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.query, root, source);
        // The pattern index and the candidate index of each syntax node that has a candidate.
        let mut chosen: HashMap<usize, (usize, usize)> = HashMap::new();
        let mut candidates = Vec::new();
        while let Some(found) = matches.next() {
            let Some(node) = found
                .captures()
                .iter()
                .find(|c| c.index == self.node_capture)
            else {
                continue;
            };
            let earlier = chosen.get(&node.node.id()).copied();
            if earlier.is_some_and(|(pattern, _)| pattern <= found.pattern_index) {
                continue;
            }
            let Some(candidate) = self.candidate(found, node.node, source) else {
                continue;
            };
            let index = match earlier {
                Some((_, index)) => {
                    candidates[index] = candidate;
                    index
                }
                None => {
                    candidates.push(candidate);
                    candidates.len() - 1
                }
            };
            chosen.insert(node.node.id(), (found.pattern_index, index));
        }
        candidates
    }

    fn candidate(
        &self,
        found: &QueryMatch<'_, '_>,
        node: Node<'_>,
        source: &[u8],
    ) -> Option<Candidate> {
        let properties = self.patterns.get(found.pattern_index)?;
        let mut label_ranges: Vec<_> = found
            .captures()
            .iter()
            .filter(|c| Some(c.index) == self.label_capture)
            .map(|c| c.node.byte_range())
            .collect();
        label_ranges.sort_by_key(|range| range.start);
        let kind = properties
            .kind
            .clone()
            .unwrap_or_else(|| node.kind().to_string());
        let range = found
            .captures()
            .iter()
            .filter(|c| Some(c.index) == self.annotation_capture)
            .map(|c| c.node.byte_range())
            .fold(node.byte_range(), |range, annotation| {
                range.start.min(annotation.start)..range.end.max(annotation.end)
            });
        Some(Candidate {
            label: label(source, &label_ranges).unwrap_or_else(|| kind.clone()),
            kind,
            range,
            label_ranges,
            level: properties.level,
            depth: iter::successors(node.parent(), |node| node.parent()).count(),
            children: Vec::new(),
        })
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
                KIND_PROPERTY if value.is_empty() => {
                    return Err(PluginError::EmptyKind { pattern });
                }
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

/// The position of the first `ERROR` or `MISSING` node below `root` in document order. A loop, not a recursion, so a
/// deep tree cannot overflow the stack.
fn syntax_error(root: Node<'_>, source: &[u8]) -> ScanError {
    let mut node = root;
    while !node.is_error() && !node.is_missing() {
        let mut cursor = node.walk();
        let Some(child) = node.children(&mut cursor).find(|child| child.has_error()) else {
            break;
        };
        node = child;
    }
    let point = node.start_position();
    let line_start = node.start_byte().saturating_sub(point.column);
    let column = source
        .get(line_start..node.start_byte())
        .map_or(point.column, |text| {
            String::from_utf8_lossy(text).chars().count()
        });
    ScanError::Syntax {
        line: point.row + 1,
        column: column + 1,
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
/// that no other contains. Of two candidates with the same range, the one with the outer syntax node is the parent.
fn nest_by_containment(mut candidates: Vec<Candidate>) -> Vec<Candidate> {
    candidates.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(b.range.end.cmp(&a.range.end))
            .then(a.depth.cmp(&b.depth))
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
        sibling.children = nest_by_level(mem::take(&mut sibling.children), sibling.range.end);
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

    fn rust(query: &str) -> Result<FormatPlugin, PluginError> {
        FormatPlugin::new(
            "test",
            "0.0.0",
            &["*.rs"],
            tree_sitter_rust::LANGUAGE.into(),
            query,
            String::new(),
        )
    }

    fn rendered(plugin: &FormatPlugin, source: &str) -> Result<String, Box<dyn Error>> {
        let nodes = plugin.scan(source.as_bytes())?;
        let mut out = String::new();
        render(&nodes, 0, &mut out);
        Ok(out)
    }

    fn outline(query: &str, source: &str) -> Result<String, Box<dyn Error>> {
        rendered(&markdown(query)?, source)
    }

    fn rust_outline(query: &str, source: &str) -> Result<String, Box<dyn Error>> {
        rendered(&rust(query)?, source)
    }

    /// The line and column of the syntax error that a scan of `source` reports.
    fn syntax_error(source: &str) -> Result<Option<(usize, usize)>, Box<dyn Error>> {
        match rust("(function_item) @node")?.scan(source.as_bytes()) {
            Err(ScanError::Syntax { line, column }) => Ok(Some((line, column))),
            Err(err) => Err(err.into()),
            Ok(_) => Ok(None),
        }
    }

    const HEADINGS: &str = r#"
((atx_heading (atx_h2_marker) heading_content: (inline) @label) @node (#set! kind "heading") (#set! level "2"))
((atx_heading (atx_h3_marker) heading_content: (inline) @label) @node (#set! kind "heading") (#set! level "3"))
((section (paragraph) @node) (#set! kind "paragraph"))
"#;

    #[test]
    fn paragraph_pattern_gives_one_node() -> Result<(), Box<dyn Error>> {
        let query = r#"((paragraph) @node (#set! kind "paragraph"))"#;
        assert_eq!(
            outline(query, "Ardent derives an outline from each source file.\n")?,
            "paragraph paragraph 0..49\n"
        );
        Ok(())
    }

    #[test]
    fn kind_defaults_to_the_syntax_node_type() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            outline("(block_quote) @node", "> quoted\n")?,
            "block_quote block_quote 0..9\n"
        );
        Ok(())
    }

    #[test]
    fn one_node_for_a_syntax_node_that_two_patterns_match() -> Result<(), Box<dyn Error>> {
        let query =
            r#"((paragraph) @node (#set! kind "first")) ((paragraph) @node (#set! kind "second"))"#;
        assert_eq!(outline(query, "Text.\n")?, "first first 0..6\n");
        Ok(())
    }

    #[test]
    fn first_pattern_wins_when_a_later_pattern_finishes_first() -> Result<(), Box<dyn Error>> {
        let query = r#"
((atx_heading (atx_h2_marker) heading_content: (inline) @label) @node (#set! kind "first"))
((atx_heading) @node (#set! kind "second"))
"#;
        assert_eq!(outline(query, "## Title\n")?, "first Title 0..9\n");
        Ok(())
    }

    #[test]
    fn nests_by_containment() -> Result<(), Box<dyn Error>> {
        let query = r#"((list) @node (#set! kind "list")) ((list_item) @node (#set! kind "item"))"#;
        assert_eq!(
            outline(query, "- a\n- b\n  - c\n")?,
            "list list 0..14\n  item item 0..4\n  item item 4..14\n    list list 10..14\n      item item 10..14\n"
        );
        Ok(())
    }

    #[test]
    fn outer_syntax_node_is_the_parent_when_two_nodes_share_a_range() -> Result<(), Box<dyn Error>>
    {
        // The list pattern finishes at the paragraph, after the item pattern has finished at the item.
        let query = r#"((list (list_item (paragraph))) @node (#set! kind "list")) ((list_item) @node (#set! kind "item"))"#;
        assert_eq!(
            outline(query, "- a\n")?,
            "list list 0..4\n  item item 0..4\n"
        );
        Ok(())
    }

    #[test]
    fn nests_by_level() -> Result<(), Box<dyn Error>> {
        let source = "## A\n\nOne.\n\n### B\n\nTwo.\n\n## C\n";
        assert_eq!(
            outline(HEADINGS, source)?,
            "heading A 0..25\n  paragraph paragraph 6..11\n  heading B 12..25\n    paragraph paragraph 19..24\nheading C 25..30\n"
        );
        Ok(())
    }

    #[test]
    fn a_skipped_level_still_nests() -> Result<(), Box<dyn Error>> {
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
    fn joins_label_captures_and_collapses_whitespace() -> Result<(), Box<dyn Error>> {
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
    fn label_without_text_is_the_kind() -> Result<(), Box<dyn Error>> {
        assert_eq!(outline(HEADINGS, "Text.\n")?, "paragraph paragraph 0..6\n");
        Ok(())
    }

    const DOC_COMMENTS: &str = "((line_comment)* @annotation . (function_item) @node)";

    #[test]
    fn annotations_widen_the_range() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            rust_outline(DOC_COMMENTS, "/// Runs.\nfn run() {}\n")?,
            "function_item function_item 0..21\n"
        );
        Ok(())
    }

    #[test]
    fn match_without_annotations_keeps_the_node_range() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            rust_outline(DOC_COMMENTS, "fn a() {}\n\nfn run() {}\n")?,
            "function_item function_item 0..9\nfunction_item function_item 11..22\n"
        );
        Ok(())
    }

    #[test]
    fn half_typed_function_is_a_syntax_error_on_its_line() -> Result<(), Box<dyn Error>> {
        let error = syntax_error("fn run() {}\nfn helper(\n")?;
        assert_eq!(error.map(|(line, _)| line), Some(2));
        Ok(())
    }

    #[test]
    fn syntax_error_on_the_first_line() -> Result<(), Box<dyn Error>> {
        assert_eq!(syntax_error("fn run() {} )\n")?, Some((1, 13)));
        Ok(())
    }

    #[test]
    fn syntax_error_column_counts_characters() -> Result<(), Box<dyn Error>> {
        assert_eq!(syntax_error("const S: &str = \"äöü\"; )\n")?, Some((1, 24)));
        Ok(())
    }

    #[test]
    fn missing_token_is_a_syntax_error() -> Result<(), Box<dyn Error>> {
        assert_eq!(syntax_error("fn f() { let x = 1 }\n")?, Some((1, 19)));
        Ok(())
    }

    #[test]
    fn file_without_errors_has_no_syntax_error() -> Result<(), Box<dyn Error>> {
        assert_eq!(syntax_error("fn run() {}\n")?, None);
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
    fn rejects_an_empty_kind() {
        for query in [
            r#"((paragraph) @node (#set! kind))"#,
            r#"((paragraph) @node (#set! kind ""))"#,
        ] {
            assert!(
                matches!(markdown(query), Err(PluginError::EmptyKind { pattern: 0 })),
                "{query}"
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
