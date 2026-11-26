use ardent_core::highlight::{Category, Highlight};

use super::{CALL_FUEL, MAX_SOURCE_BYTES, Runtime, RuntimeError};

wasmtime::component::bindgen!({ path: "../../wit", world: "syntax-highlighting" });
use exports::ardent::format::highlighter::{Category as GuestCategory, Span};

impl Runtime {
    pub(crate) fn highlight(&mut self, source: &[u8]) -> Result<Vec<Highlight>, RuntimeError> {
        self.highlight_with_fuel(source, CALL_FUEL)
    }

    fn highlight_with_fuel(
        &mut self,
        source: &[u8],
        fuel: u64,
    ) -> Result<Vec<Highlight>, RuntimeError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(RuntimeError::Invalid(
                "source exceeds highlight input limit",
            ));
        }
        let spans = self.call(|i| {
            if i.component
                .get_export_index(&mut i.store, None, "ardent:format/highlighter@0.1.0")
                .is_none()
            {
                return Ok(Vec::new());
            }
            let bindings = SyntaxHighlighting::new(&mut i.store, &i.component)?;
            i.store.set_fuel(fuel)?;
            bindings
                .ardent_format_highlighter()
                .call_highlight(&mut i.store, source)
        })?;
        validate(spans, source.len())
    }
}

fn validate(spans: Vec<Span>, size: usize) -> Result<Vec<Highlight>, RuntimeError> {
    let mut end = 0;
    spans
        .into_iter()
        .map(|span| {
            let range = span.start as usize..span.end as usize;
            if range.is_empty() || range.start < end || range.end > size {
                return Err(RuntimeError::Invalid("invalid highlight range"));
            }
            end = range.end;
            let category = match span.category {
                GuestCategory::Keyword => Category::Keyword,
                GuestCategory::StringLiteral => Category::StringLiteral,
                GuestCategory::Comment => Category::Comment,
                GuestCategory::Heading => Category::Heading,
                GuestCategory::Code => Category::Code,
            };
            Ok(Highlight { range, category })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resets_the_instance_after_a_highlight_trap() -> Result<(), Box<dyn std::error::Error>> {
        let bytes = include_bytes!("../../../../plugins/rust/format.wasm");
        let mut runtime = Runtime::compile(bytes)?;
        let source = b"fn sample() {}";
        assert!(runtime.highlight_with_fuel(source, 0).is_err());
        assert!(runtime.active.is_none());
        assert_eq!(runtime.highlight(source)?[0].range, 0..2);
        assert_eq!(runtime.scan("sample.rs", source)?.len(), 1);
        Ok(())
    }

    #[test]
    fn validates_all_ranges_before_returning_styles() {
        let span = |start, end| Span {
            start,
            end,
            category: GuestCategory::Code,
        };
        for invalid in [span(2, 3), span(3, 3), span(4, 3), span(3, 6), span(0, 1)] {
            assert!(validate(vec![span(1, 3), invalid], 5).is_err());
        }
        assert_eq!(
            validate(vec![span(1, 3), span(3, 5)], 5)
                .expect("adjacent ranges")
                .len(),
            2
        );
    }
}
