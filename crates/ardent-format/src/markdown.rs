//! The built-in Markdown format plugin (spec `markdown-format`). Its grammar links natively through `tree-sitter-md`,
//! and its outline query is the file `markdown/outline.scm`.

use crate::engine::{self, FormatPlugin, PluginError};

pub const NAME: &str = "markdown";
const VERSION: &str = "0.1.0";
const DEFAULT_FILES: &[&str] = &["*.md", "*.markdown"];
const QUERY: &str = include_str!("markdown/outline.scm");
/// The grammar crate and its version, which `Cargo.toml` pins exactly, for the format hash.
const GRAMMAR_CRATE: &str = "tree-sitter-md";
const GRAMMAR_VERSION: &str = "0.5.4";

pub fn plugin() -> Result<FormatPlugin, PluginError> {
    FormatPlugin::new(
        NAME,
        VERSION,
        DEFAULT_FILES,
        tree_sitter_md::LANGUAGE.into(),
        QUERY,
        engine::builtin_format_hash(QUERY, GRAMMAR_CRATE, GRAMMAR_VERSION),
    )
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use ardent_core::outline::{Node, Outline};

    use super::*;

    #[test]
    fn grammar_version_is_the_pinned_version() {
        let pin = format!("{GRAMMAR_CRATE} = \"={GRAMMAR_VERSION}\"");
        assert!(include_str!("../Cargo.toml").contains(&pin), "{pin}");
    }

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
    fn identity() -> TestResult {
        let plugin = plugin()?;
        assert_eq!(
            (plugin.name.as_str(), plugin.version.as_str()),
            ("markdown", "0.1.0")
        );
        assert_eq!(plugin.default_files, ["*.md", "*.markdown"]);
        assert_eq!(plugin.format_hash.len(), 64);
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
                "docs/spec.md#paragraph | paragraph | 0..49 | 986a9a5665640a09",
                "docs/spec.md#Sync rules | heading | 50..172 | 6afe874c4163626f",
                "docs/spec.md#Sync rules/paragraph | paragraph | 65..99 | c066d35079ec2897",
                "docs/spec.md#Sync rules/Renames | heading | 100..172 | f2009c466ed1d13a",
                "docs/spec.md#Sync rules/Renames/paragraph | paragraph | 113..171 | 0b019916628c0b2b",
                "docs/spec.md#Example | heading | 172..201 | 5547ffff36732ee1",
                "docs/spec.md#Example/paragraph | paragraph | 184..201 | 7b8f8a6fa531fab0",
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
                "guide.md#Setup/paragraph",
                "guide.md#Usage"
            ]
        );
        Ok(())
    }

    #[test]
    fn heading_without_text() -> TestResult {
        assert_eq!(paths("notes.md", "##\n")?, ["notes.md", "notes.md#heading"]);
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
                "guide.md#New/paragraph"
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
                "README.md#Install/paragraph"
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
                "README.md#list",
                "README.md#list/item",
                "README.md#list/item/Note",
                "README.md#list/item/Note/paragraph",
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
                "README.md#Install/list",
                "README.md#Install/list/item",
                "README.md#Install/list/item[2]",
                "README.md#Install/list/item[2]/list",
                "README.md#Install/list/item[2]/list/item",
                "README.md#Install/list/item[2]/list/item[2]",
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
                "README.md#Install/code",
                "README.md#Install/code[2]"
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
                "README.md#Notes/quote",
                "README.md#Notes/table"
            ]
        );
        Ok(())
    }

    #[test]
    fn front_matter() -> TestResult {
        let yaml = outline("guide.md", "---\ntitle: Guide\n---\n\n# Guide\n")?;
        assert_eq!(
            node(&yaml, "guide.md#front_matter").map(|n| n.range.clone()),
            Some(0..21)
        );
        let toml = outline("guide.md", "+++\ntitle = \"Guide\"\n+++\n")?;
        assert_eq!(
            node(&toml, "guide.md#front_matter").map(|n| n.kind.as_str()),
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
                "README.md#Notes/paragraph",
                "README.md#Notes/paragraph[2]"
            ]
        );
        assert_eq!(
            node(&outline, "README.md#Notes").map(|n| n.range.clone()),
            Some(0..text.len())
        );
        Ok(())
    }
}
