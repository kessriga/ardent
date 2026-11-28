//! The Markdown format component.

#![cfg(any(test, target_arch = "wasm32"))]

use ardent_format_guest::{self as guest, PluginError, QueryEngine};
use guest::scanner::{Guest, Metadata, Node, ScanError};

mod highlight;
mod properties;

struct Component;

fn plugin() -> Result<QueryEngine, PluginError> {
    QueryEngine::new(tree_sitter_md::LANGUAGE.into(), include_str!("outline.scm"))
}

thread_local! {
    static ENGINE: Result<QueryEngine, PluginError> = plugin();
}

impl guest::query::Guest for Component {
    fn prepare() {
        ENGINE.with(|_| ());
    }
}

impl guest::highlighting::Guest for Component {
    fn highlight(source: Vec<u8>) -> Vec<guest::highlighting::Span> {
        highlight::spans(&source)
    }
}

impl guest::properties::Guest for Component {
    fn properties(source: Vec<u8>) -> Vec<guest::properties::Group> {
        ENGINE.with(|engine| {
            engine
                .as_ref()
                .ok()
                .and_then(|engine| engine.properties(&source, properties::read).ok())
                .unwrap_or_default()
        })
    }
}

impl Guest for Component {
    fn describe() -> Metadata {
        guest::metadata(
            "markdown",
            env!("CARGO_PKG_VERSION"),
            &["*.md", "*.markdown"],
        )
    }

    fn scan(source: Vec<u8>) -> Result<Vec<Node>, ScanError> {
        ENGINE.with(|engine| guest::scan(engine.as_ref(), &source))
    }
}

#[cfg(target_arch = "wasm32")]
guest::export!(Component with_types_in guest);
#[cfg(target_arch = "wasm32")]
guest::query::export!(Component with_types_in guest::query);
#[cfg(target_arch = "wasm32")]
guest::highlighting::export!(Component with_types_in guest::highlighting);
#[cfg(target_arch = "wasm32")]
guest::properties::export!(Component with_types_in guest::properties);

#[cfg(test)]
mod tests {
    use std::error::Error;

    use ardent_core::outline::{Node, Outline};

    use super::*;

    type TestResult = Result<(), Box<dyn Error>>;

    fn outline(source_path: &str, text: &str) -> Result<Outline, Box<dyn Error>> {
        let top_level = plugin()?.scan(text.as_bytes())?;
        Ok(Outline::build(source_path, text.as_bytes(), &top_level))
    }

    fn paths(source_path: &str, text: &str) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(outline(source_path, text)?
            .nodes
            .into_iter()
            .map(|node| node.path)
            .collect())
    }

    fn node<'a>(outline: &'a Outline, path: &str) -> Option<&'a Node> {
        outline.nodes.iter().find(|node| node.path == path)
    }

    #[test]
    fn checkbox_belongs_to_the_item_with_the_marker() -> TestResult {
        use guest::properties::Guest as _;

        let text = "- [x] done\n- [ ] pending\n- plain\n  - [X] nested\n";
        let outline = outline("a.md", text)?;
        let groups = Component::properties(text.as_bytes().to_vec());
        let rows: Vec<_> = groups
            .iter()
            .flat_map(|g| {
                let item = &outline.nodes[g.node as usize + 1];
                g.properties.iter().map(move |p| {
                    (
                        item.path.as_str(),
                        p.name.as_str(),
                        p.value.as_str(),
                        p.inline,
                    )
                })
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("a.md#list.1/item.1", "checked", "true", true),
                ("a.md#list.1/item.2", "checked", "false", true),
                ("a.md#list.1/item.3/list.1/item.1", "checked", "true", true),
            ]
        );
        Ok(())
    }

    #[test]
    fn identity() -> TestResult {
        let meta = Component::describe();
        assert_eq!(
            (meta.name.as_str(), meta.version.as_str()),
            ("markdown", "0.1.0")
        );
        assert_eq!(meta.files, ["*.md", "*.markdown"]);
        assert!(meta.prefixes.is_empty());
        Ok(())
    }

    #[test]
    fn structure_of_the_spec_example() -> TestResult {
        let text = "Ardent derives an outline from each source file.\n\n## Sync rules\n\nEvery command starts with a \
                    sync.\n\n### Renames\n\nA rename keeps the fingerprint.\nIt also keeps every link.\n\n## Example\n\n\
                    A short example.\n";
        let rows: Vec<String> = outline("docs/spec.md", text)?
            .nodes
            .iter()
            .map(|n| {
                format!(
                    "{} | {} | {}..{} | {}",
                    n.path, n.kind, n.range.start, n.range.end, n.fingerprint
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                "docs/spec.md | file | 0..201 | 64d285e69608de32",
                "docs/spec.md#paragraph.1 | paragraph | 0..49 | 986a9a5665640a09",
                "docs/spec.md#Sync rules | heading | 50..172 | 6afe874c4163626f",
                "docs/spec.md#Sync rules/paragraph.1 | paragraph | 65..99 | c066d35079ec2897",
                "docs/spec.md#Sync rules/Renames | heading | 100..172 | f2009c466ed1d13a",
                "docs/spec.md#Sync rules/Renames/paragraph.1 | paragraph | 113..171 | 0b019916628c0b2b",
                "docs/spec.md#Example | heading | 172..201 | 5547ffff36732ee1",
                "docs/spec.md#Example/paragraph.1 | paragraph | 184..201 | 7b8f8a6fa531fab0",
            ]
        );
        Ok(())
    }

    #[test]
    fn setext_heading_nests_its_content() -> TestResult {
        assert_eq!(
            paths("guide.md", "Setup\n---\n\nRun it.\n\n## Usage\n")?,
            [
                "guide.md",
                "guide.md#Setup",
                "guide.md#Setup/paragraph.1",
                "guide.md#Usage"
            ]
        );
        Ok(())
    }

    #[test]
    fn heading_without_text() -> TestResult {
        assert_eq!(
            paths("notes.md", "##\n")?,
            ["notes.md", "notes.md#heading.1"]
        );
        Ok(())
    }

    #[test]
    fn skipped_heading_level() -> TestResult {
        assert_eq!(
            paths("guide.md", "# Guide\n### Details\n## Install\n")?,
            [
                "guide.md",
                "guide.md#Guide",
                "guide.md#Guide/Details",
                "guide.md#Guide/Install"
            ]
        );
        Ok(())
    }

    #[test]
    fn setext_heading_of_level_1_inside_an_atx_section() -> TestResult {
        assert_eq!(
            paths("guide.md", "## Old\n\nNew\n===\n\nText.\n")?,
            [
                "guide.md",
                "guide.md#Old",
                "guide.md#New",
                "guide.md#New/paragraph.1"
            ]
        );
        Ok(())
    }

    #[test]
    fn paragraph_under_a_heading() -> TestResult {
        assert_eq!(
            paths("README.md", "## Install\n\nRun the installer.\n")?,
            [
                "README.md",
                "README.md#Install",
                "README.md#Install/paragraph.1"
            ]
        );
        Ok(())
    }

    #[test]
    fn text_of_a_list_item_is_not_a_node() -> TestResult {
        let outline = outline("README.md", "- Download the binary.\n")?;
        let kinds: Vec<_> = outline
            .nodes
            .iter()
            .map(|node| node.kind.as_str())
            .collect();
        assert_eq!(kinds, ["file", "list", "item"]);
        Ok(())
    }

    #[test]
    fn paragraph_under_a_heading_inside_a_list_item() -> TestResult {
        assert_eq!(
            paths("README.md", "- ## Note\n\n  Text below the note.\n")?,
            [
                "README.md",
                "README.md#list.1",
                "README.md#list.1/item.1",
                "README.md#list.1/item.1/Note",
                "README.md#list.1/item.1/Note/paragraph.1",
            ]
        );
        Ok(())
    }

    #[test]
    fn nested_list() -> TestResult {
        let text = "## Install\n\n- Download.\n- Put it on the path:\n  - macOS\n  - Linux\n\n  Then restart.\n";
        assert_eq!(
            paths("README.md", text)?,
            [
                "README.md",
                "README.md#Install",
                "README.md#Install/list.1",
                "README.md#Install/list.1/item.1",
                "README.md#Install/list.1/item.2",
                "README.md#Install/list.1/item.2/list.1",
                "README.md#Install/list.1/item.2/list.1/item.1",
                "README.md#Install/list.1/item.2/list.1/item.2",
            ]
        );
        Ok(())
    }

    #[test]
    fn ordered_list() -> TestResult {
        let outline = outline("README.md", "## Install\n\n1. First\n2. Second\n")?;
        let kinds: Vec<_> = outline
            .nodes
            .iter()
            .map(|node| node.kind.as_str())
            .collect();
        assert_eq!(kinds, ["file", "heading", "list", "item", "item"]);
        Ok(())
    }

    #[test]
    fn code_blocks() -> TestResult {
        let text = "## Install\n\n```sh\ncargo install ardent\n```\n\n    indented code\n";
        assert_eq!(
            paths("README.md", text)?,
            [
                "README.md",
                "README.md#Install",
                "README.md#Install/code.1",
                "README.md#Install/code.2"
            ]
        );
        Ok(())
    }

    #[test]
    fn quote_and_table_under_a_heading() -> TestResult {
        let text = "## Notes\n\n> A quote.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n";
        assert_eq!(
            paths("README.md", text)?,
            [
                "README.md",
                "README.md#Notes",
                "README.md#Notes/quote.1",
                "README.md#Notes/table.1"
            ]
        );
        Ok(())
    }

    #[test]
    fn front_matter() -> TestResult {
        let yaml = outline("guide.md", "---\ntitle: Guide\n---\n\n# Guide\n")?;
        assert_eq!(
            node(&yaml, "guide.md#front_matter.1").map(|n| n.range.clone()),
            Some(0..21)
        );
        let toml = outline("guide.md", "+++\ntitle = \"Guide\"\n+++\n")?;
        assert_eq!(
            node(&toml, "guide.md#front_matter.1").map(|n| n.kind.as_str()),
            Some("front_matter")
        );
        Ok(())
    }

    #[test]
    fn thematic_break_is_not_a_node() -> TestResult {
        let text = "## Notes\n\nOne.\n\n---\n\nTwo.\n";
        let outline = outline("README.md", text)?;
        let paths: Vec<_> = outline
            .nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "README.md",
                "README.md#Notes",
                "README.md#Notes/paragraph.1",
                "README.md#Notes/paragraph.2"
            ]
        );
        assert_eq!(
            node(&outline, "README.md#Notes").map(|n| n.range.clone()),
            Some(0..text.len())
        );
        Ok(())
    }
}
