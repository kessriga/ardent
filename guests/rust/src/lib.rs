//! The Rust format component.

#![cfg(any(test, target_arch = "wasm32"))]

use ardent_format_guest::{self as guest, PluginError, QueryEngine};
use guest::scanner::{Guest, Metadata, Node, ScanError};

struct Component;

fn plugin() -> Result<QueryEngine, PluginError> {
    QueryEngine::new(
        tree_sitter_rust::LANGUAGE.into(),
        include_str!("outline.scm"),
    )
}

impl Guest for Component {
    fn describe() -> Metadata {
        guest::metadata("rust", env!("CARGO_PKG_VERSION"), &["*.rs"])
    }

    fn scan(source: Vec<u8>) -> Result<Vec<Node>, ScanError> {
        thread_local! {
            static ENGINE: Result<QueryEngine, PluginError> = plugin();
        }
        ENGINE.with(|engine| guest::scan(engine.as_ref(), &source))
    }
}

#[cfg(target_arch = "wasm32")]
guest::export!(Component with_types_in guest);

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::ops::Range;

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

    fn range(source_path: &str, text: &str, path: &str) -> Result<Range<usize>, Box<dyn Error>> {
        let outline = outline(source_path, text)?;
        let node = node(&outline, path).ok_or_else(|| format!("no node {path}"))?;
        Ok(node.range.clone())
    }

    /// The byte offset of `part` in `text`.
    fn offset(text: &str, part: &str) -> Result<usize, Box<dyn Error>> {
        Ok(text.find(part).ok_or_else(|| format!("no {part:?}"))?)
    }

    #[test]
    fn identity() -> TestResult {
        let meta = Component::describe();
        assert_eq!(
            (meta.name.as_str(), meta.version.as_str()),
            ("rust", "0.1.0")
        );
        assert_eq!(meta.files, ["*.rs"]);
        assert!(meta.prefixes.is_empty());
        Ok(())
    }

    const ENGINE: &str = "pub struct Engine {\n    files: Vec<String>,\n}\n\nimpl Engine {\n    pub fn run(&self) -> \
                          usize {\n        self.files.len()\n    }\n\n    fn reset(&mut self) {\n        \
                          self.files.clear();\n    }\n}\n";

    #[test]
    fn structure_of_the_spec_example() -> TestResult {
        let outline = outline("src/engine.rs", ENGINE)?;
        let rows: Vec<String> = outline
            .nodes
            .iter()
            .map(|n| {
                format!(
                    "{} | {} | {}..{} | {}",
                    n.path, n.kind, n.range.start, n.range.end, n.fingerprint
                )
            })
            .collect();
        assert_eq!(ENGINE.len(), 188);
        assert_eq!(rows.len(), 6, "{rows:#?}");
        for row in [
            "src/engine.rs | file | 0..188 | 174add0298423e83",
            "src/engine.rs#impl Engine | impl | 47..187 | e756306dadb3fb00",
            "src/engine.rs#impl Engine/fn run | fn | 65..124 | a33b27c3382d2785",
            "src/engine.rs#impl Engine/fn reset | fn | 130..185 | 0c3892e426b726a6",
        ] {
            assert!(rows.iter().any(|r| r == row), "{row}\n{rows:#?}");
        }
        assert_eq!(
            node(&outline, "src/engine.rs#struct Engine").map(|n| n.kind.as_str()),
            Some("struct")
        );
        assert_eq!(
            node(&outline, "src/engine.rs#struct Engine/files").map(|n| n.kind.as_str()),
            Some("field")
        );
        Ok(())
    }

    #[test]
    fn item_kinds() -> TestResult {
        let text = "mod m {}\nimpl S {}\ntrait T {}\nfn f() {}\nstruct S;\nenum E {}\nunion U { a: u8 }\n\
                    const C: u8 = 1;\nstatic V: u8 = 1;\ntype A = u8;\nmacro_rules! m { () => {} }\nextern \"C\" {}\n";
        let outline = outline("src/lib.rs", text)?;
        let kinds: Vec<_> = outline
            .nodes
            .iter()
            .filter(|node| node.kind != "field")
            .map(|node| node.kind.as_str())
            .collect();
        assert_eq!(
            kinds,
            [
                "file", "mod", "impl", "trait", "fn", "struct", "enum", "union", "const", "static",
                "type", "macro", "extern"
            ]
        );
        Ok(())
    }

    #[test]
    fn item_labels() -> TestResult {
        let text = "mod m {}\ntrait Scan {}\nstruct Engine;\nenum Kind {}\nunion Bits { i: u32 }\n\
                    const MAX: usize = 3;\nstatic mut COUNT: u32 = 0;\ntype Alias = u8;\n\
                    macro_rules! log { () => {} }\n";
        let top_level: Vec<_> = paths("src/lib.rs", text)?
            .into_iter()
            .filter(|path| path != "src/lib.rs#union Bits/i")
            .collect();
        assert_eq!(
            top_level,
            [
                "src/lib.rs",
                "src/lib.rs#mod m",
                "src/lib.rs#trait Scan",
                "src/lib.rs#struct Engine",
                "src/lib.rs#enum Kind",
                "src/lib.rs#union Bits",
                "src/lib.rs#const MAX",
                "src/lib.rs#static COUNT",
                "src/lib.rs#type Alias",
                "src/lib.rs#macro_rules! log",
            ]
        );
        Ok(())
    }

    #[test]
    fn declared_module() -> TestResult {
        let outline = outline("src/lib.rs", "mod engine;\n")?;
        assert_eq!(
            node(&outline, "src/lib.rs#mod engine").map(|n| n.kind.as_str()),
            Some("mod")
        );
        Ok(())
    }

    #[test]
    fn function_inside_a_function_body() -> TestResult {
        assert_eq!(
            paths(
                "src/lib.rs",
                "fn run() {\n    fn step() {}\n    step();\n}\n"
            )?,
            [
                "src/lib.rs",
                "src/lib.rs#fn run",
                "src/lib.rs#fn run/fn step"
            ]
        );
        Ok(())
    }

    #[test]
    fn public_async_function_with_generics() -> TestResult {
        assert_eq!(
            paths(
                "src/lib.rs",
                "pub async fn fetch<T: Send>(url: &str) -> T {\n    todo!()\n}\n"
            )?,
            ["src/lib.rs", "src/lib.rs#fn fetch"]
        );
        Ok(())
    }

    #[test]
    fn two_functions_with_one_name() -> TestResult {
        let text = "#[cfg(unix)]\nfn home() {}\n\n#[cfg(windows)]\nfn home() {}\n";
        assert_eq!(
            paths("src/lib.rs", text)?,
            ["src/lib.rs", "src/lib.rs#fn home", "src/lib.rs#fn home[2]"]
        );
        Ok(())
    }

    #[test]
    fn trait_impl() -> TestResult {
        let text = "impl<T: Clone> From<T> for Engine where T: Send {\n    fn from(t: T) -> Self {\n        \
                    todo!()\n    }\n}\n";
        assert_eq!(
            paths("src/engine.rs", text)?,
            [
                "src/engine.rs",
                "src/engine.rs#impl From<T> for Engine",
                "src/engine.rs#impl From<T> for Engine/fn from"
            ]
        );
        Ok(())
    }

    #[test]
    fn inherent_impl_and_trait_impl_of_one_type() -> TestResult {
        let text =
            "impl Engine {\n    fn new() {}\n}\n\nimpl Default for Engine {\n    fn new() {}\n}\n";
        assert_eq!(
            paths("src/engine.rs", text)?,
            [
                "src/engine.rs",
                "src/engine.rs#impl Engine",
                "src/engine.rs#impl Engine/fn new",
                "src/engine.rs#impl Default for Engine",
                "src/engine.rs#impl Default for Engine/fn new"
            ]
        );
        Ok(())
    }

    #[test]
    fn negative_and_unsafe_impls() -> TestResult {
        let text = "impl !Send for Engine {}\nunsafe impl Sync for Engine {}\n";
        assert_eq!(
            paths("src/engine.rs", text)?,
            [
                "src/engine.rs",
                "src/engine.rs#impl Send for Engine",
                "src/engine.rs#impl Sync for Engine"
            ]
        );
        Ok(())
    }

    #[test]
    fn function_in_an_extern_block() -> TestResult {
        let text = "extern \"C\" {\n    fn abs(x: i32) -> i32;\n}\n\nextern {}\n";
        assert_eq!(
            paths("src/ffi.rs", text)?,
            [
                "src/ffi.rs",
                "src/ffi.rs#extern \"C\"",
                "src/ffi.rs#extern \"C\"/fn abs",
                "src/ffi.rs#extern"
            ]
        );
        Ok(())
    }

    #[test]
    fn function_in_an_inline_module() -> TestResult {
        assert_eq!(
            paths("src/lib.rs", "mod tests {\n    fn helper() {}\n}\n")?,
            [
                "src/lib.rs",
                "src/lib.rs#mod tests",
                "src/lib.rs#mod tests/fn helper"
            ]
        );
        Ok(())
    }

    #[test]
    fn items_of_a_trait() -> TestResult {
        let text = "trait Scan {\n    type Output;\n    const LIMIT: usize;\n    fn scan(&self) -> \
                    Self::Output;\n}\n";
        assert_eq!(
            paths("src/scan.rs", text)?,
            [
                "src/scan.rs",
                "src/scan.rs#trait Scan",
                "src/scan.rs#trait Scan/type Output",
                "src/scan.rs#trait Scan/const LIMIT",
                "src/scan.rs#trait Scan/fn scan"
            ]
        );
        Ok(())
    }

    #[test]
    fn associated_items_of_an_impl() -> TestResult {
        let text = "impl Scan for Engine {\n    type Output = usize;\n    const LIMIT: usize = 3;\n    fn \
                    scan(&self) -> usize {\n        0\n    }\n}\n";
        let outline = outline("src/scan.rs", text)?;
        let kinds: Vec<_> = outline
            .nodes
            .iter()
            .map(|node| node.kind.as_str())
            .collect();
        assert_eq!(kinds, ["file", "impl", "type", "const", "fn"]);
        Ok(())
    }

    #[test]
    fn named_field() -> TestResult {
        let outline = outline(
            "src/engine.rs",
            "pub struct Engine { files: Vec<String> }\n",
        )?;
        assert_eq!(
            node(&outline, "src/engine.rs#struct Engine/files").map(|n| n.kind.as_str()),
            Some("field")
        );
        Ok(())
    }

    #[test]
    fn enum_with_a_struct_variant_and_a_tuple_variant() -> TestResult {
        let text =
            "enum Entry {\n    File,\n    Dir { path: PathBuf },\n    Link(PathBuf, bool),\n}\n";
        let outline = outline("src/entry.rs", text)?;
        let rows: Vec<_> = outline
            .nodes
            .iter()
            .map(|node| format!("{} {}", node.kind, node.path))
            .collect();
        assert_eq!(
            rows,
            [
                "file src/entry.rs",
                "enum src/entry.rs#enum Entry",
                "variant src/entry.rs#enum Entry/File",
                "variant src/entry.rs#enum Entry/Dir",
                "field src/entry.rs#enum Entry/Dir/path",
                "variant src/entry.rs#enum Entry/Link",
                "field src/entry.rs#enum Entry/Link/field",
                "field src/entry.rs#enum Entry/Link/field[2]"
            ]
        );
        Ok(())
    }

    #[test]
    fn fields_of_a_union_and_a_tuple_struct() -> TestResult {
        let text =
            "union Bits {\n    i: u32,\n    f: f32,\n}\n\nstruct Pair(pub u8, Vec<String>);\n";
        assert_eq!(
            paths("src/lib.rs", text)?,
            [
                "src/lib.rs",
                "src/lib.rs#union Bits",
                "src/lib.rs#union Bits/i",
                "src/lib.rs#union Bits/f",
                "src/lib.rs#struct Pair",
                "src/lib.rs#struct Pair/field",
                "src/lib.rs#struct Pair/field[2]"
            ]
        );
        Ok(())
    }

    #[test]
    fn attribute_on_a_tuple_field() -> TestResult {
        let text = "struct Id(#[serde(skip)] pub u64);\n";
        let field = range("src/id.rs", text, "src/id.rs#struct Id/field")?;
        assert_eq!(field, offset(text, "#[serde")?..offset(text, ");")?);
        Ok(())
    }

    #[test]
    fn documented_struct() -> TestResult {
        let text = "/// The engine.\n#[derive(Debug)]\npub struct Engine;\n";
        assert_eq!(
            range("src/engine.rs", text, "src/engine.rs#struct Engine")?,
            0..51
        );
        Ok(())
    }

    #[test]
    fn block_doc_comment_and_blank_lines() -> TestResult {
        let text = "/** Runs. */\n\n#[inline]\n\nfn run() {}\n";
        assert_eq!(range("src/lib.rs", text, "src/lib.rs#fn run")?.start, 0);
        Ok(())
    }

    #[test]
    fn edited_doc_comment_changes_the_fingerprint() -> TestResult {
        let fingerprint = |doc: &str| -> Result<String, Box<dyn Error>> {
            let text = format!("impl Engine {{\n    /// {doc}\n    fn run() {{}}\n}}\n");
            let outline = outline("src/engine.rs", &text)?;
            let run = node(&outline, "src/engine.rs#impl Engine/fn run").ok_or("no fn run")?;
            Ok(run.fingerprint.clone())
        };
        assert_ne!(fingerprint("Runs.")?, fingerprint("Runs it.")?);
        Ok(())
    }

    #[test]
    fn ordinary_comment_between_a_doc_comment_and_its_item() -> TestResult {
        let text = "/// Runs it.\n// TODO: faster\nfn run() {}\n";
        assert_eq!(
            range("src/lib.rs", text, "src/lib.rs#fn run")?.start,
            offset(text, "fn run")?
        );
        Ok(())
    }

    #[test]
    fn inner_doc_comments_and_attributes_stay_with_the_container() -> TestResult {
        let text = "//! Crate.\n#![allow(dead_code)]\n\nmod engine {\n    //! Engine.\n    #![allow(unused)]\n    \
                    fn run() {}\n}\n";
        let outline = outline("src/lib.rs", text)?;
        let start = |path: &str| node(&outline, path).map(|n| n.range.start);
        assert_eq!(
            start("src/lib.rs#mod engine"),
            Some(offset(text, "mod engine")?)
        );
        assert_eq!(
            start("src/lib.rs#mod engine/fn run"),
            Some(offset(text, "fn run")?)
        );
        Ok(())
    }

    #[test]
    fn documented_field_and_variant() -> TestResult {
        let text = "struct S {\n    /// Doc.\n    #[serde(skip)]\n    pub a: u8,\n}\n\nenum K {\n    /// A \
                    file.\n    File,\n}\n";
        assert_eq!(
            range("src/lib.rs", text, "src/lib.rs#struct S/a")?.start,
            offset(text, "/// Doc.")?
        );
        assert_eq!(
            range("src/lib.rs", text, "src/lib.rs#enum K/File")?.start,
            offset(text, "/// A")?
        );
        Ok(())
    }

    #[test]
    fn imports_and_statements_are_not_nodes() -> TestResult {
        let text = "use std::fs;\nextern crate alloc;\n\nthread_local! { static X: u8 = 1; }\n\nfn main() {\n    \
                    let text = fs::read_to_string(\"a\").unwrap();\n}\n";
        assert_eq!(
            paths("src/main.rs", text)?,
            ["src/main.rs", "src/main.rs#fn main"]
        );
        Ok(())
    }
}
