//! Snapshot tests: the outline of each Rust file in `tests/rust/`, as the `nodes` of its structure file, and the error
//! for a file with a syntax error.

use std::error::Error;
use std::fs;
use std::path::Path;

use ardent_core::outline::Outline;
use ardent_core::yaml;
use ardent_format::rust;

fn outline_yaml(name: &str) -> Result<String, Box<dyn Error>> {
    let source = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/rust")
            .join(name),
    )?;
    let top_level = rust::plugin()?.scan(&source)?;
    Ok(yaml::to_string(&Outline::build(name, &source, &top_level))?)
}

#[test]
fn inline_modules_and_items_in_function_bodies() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(outline_yaml("modules.rs")?);
    Ok(())
}

#[test]
fn structs_enums_traits_and_impl_blocks() -> Result<(), Box<dyn Error>> {
    insta::assert_snapshot!(outline_yaml("types.rs")?);
    Ok(())
}

/// The file stays inline, so that a sync of this repository finds no Rust file with a syntax error.
#[test]
fn syntax_error() -> Result<(), Box<dyn Error>> {
    let source =
        "impl Engine {\n    pub fn run(&self) {}\n\n    fn helper(\n\n    fn after(&self) {}\n}\n";
    let Err(err) = rust::plugin()?.scan(source.as_bytes()) else {
        return Err("the scan found no syntax error".into());
    };
    insta::assert_snapshot!(err.to_string());
    Ok(())
}
