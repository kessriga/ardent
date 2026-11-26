//! Snapshot tests: the outline of each Markdown file in `tests/markdown/`, as the `nodes` of its structure file.

mod common;

use std::error::Error;
use std::fs;
use std::path::Path;

use ardent_core::outline::Outline;
use ardent_core::yaml;

fn outline_yaml(name: &str) -> Result<String, Box<dyn Error>> {
    let source = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/markdown")
            .join(name),
    )?;
    let top_level = common::plugin("markdown")?.scan(name, &source)?;
    Ok(yaml::to_string(&Outline::build(name, &source, &top_level))?)
}

#[test]
fn guide_with_front_matter_nested_headings_lists_and_code() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(outline_yaml("guide.md")?);
    Ok(())
}

#[test]
fn setext_headings() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(outline_yaml("setext.md")?);
    Ok(())
}

#[test]
fn quotes_tables_and_blocks_that_are_not_nodes() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(outline_yaml("blocks.md")?);
    Ok(())
}

#[test]
fn packaged_syntax_spans() -> Result<(), Box<dyn Error>> {
    use ardent_core::highlight::Category::{Code, Heading};
    let source =
        "# Try `this` now\n\nBody `let`.\n\nNext\n----\n\n```rs\nfn next() {}\n```\n\n    code\n";
    let expected = [
        ("# Try ", Heading),
        ("`this`", Code),
        (" now\n", Heading),
        ("`let`", Code),
        ("Next\n----\n", Heading),
        ("```rs\nfn next() {}\n```\n", Code),
        ("    code\n", Code),
    ];
    let spans = common::plugin("markdown")?.highlight(source.as_bytes())?;
    let actual: Vec<_> = spans
        .into_iter()
        .map(|span| (&source[span.range], span.category))
        .collect();
    assert_eq!(actual, expected);
    Ok(())
}
