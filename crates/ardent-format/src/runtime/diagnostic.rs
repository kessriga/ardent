use std::fmt::{self, Display, Write};

const DIAGNOSTIC_BYTES: usize = 1024;

/// Formats component diagnostics into one line and stops the producer as soon as the message is full.
pub(super) fn summary(value: impl Display) -> String {
    let mut line = Line(String::new());
    let _ = write!(line, "{value}");
    line.0
}

pub(super) fn runtime(error: &wasmtime::Error) -> String {
    if error.downcast_ref::<wasmtime::WasmBacktrace>().is_some() {
        return summary(error.root_cause());
    }
    summary(format_args!("{error:#}"))
}

struct Line(String);

impl Write for Line {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for ch in text.chars() {
            match ch {
                '"' | '\'' | '\\' => self.0.push(ch),
                _ => self.0.extend(ch.escape_debug()),
            }
            if self.0.len() > DIAGNOSTIC_BYTES {
                self.0
                    .truncate(self.0.floor_char_boundary(DIAGNOSTIC_BYTES - 3));
                self.0.push_str("...");
                return Err(fmt::Error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeError;

    #[test]
    fn summaries_fit_one_line_and_preserve_unicode() {
        let quoted_path = r#"can't parse "C:\src\main.rs""#;
        assert_eq!(summary(quoted_path), quoted_path);
        assert_eq!(summary("é\r\n\u{1b}é"), r"é\r\n\u{1b}é");
        let fits = "é".repeat(DIAGNOSTIC_BYTES / 2);
        assert_eq!(summary(&fits), fits);
        assert_eq!(
            summary(format_args!("{fits}é")),
            format!("{}...", "é".repeat((DIAGNOSTIC_BYTES - 3) / 2))
        );
    }

    #[test]
    fn stops_formatting_as_soon_as_the_budget_is_spent() {
        let reached_tail = std::cell::Cell::new(false);
        let text = "x".repeat(DIAGNOSTIC_BYTES + 1);
        let value = fmt::from_fn(|out| {
            out.write_str(&text)?;
            reached_tail.set(true);
            out.write_str("tail")
        });
        assert!(summary(value).ends_with("..."));
        assert!(!reached_tail.get());
    }

    #[test]
    fn runtime_errors_keep_context_within_the_limit() {
        let cause = format!("line\n{}", "é".repeat(DIAGNOSTIC_BYTES));
        let error = wasmtime::Error::msg(cause).context("loading scanner");
        let rendered = RuntimeError::from(error).to_string();
        let message = rendered.strip_prefix("component runtime: ");
        assert!(
            message.is_some_and(|text| text.starts_with(r"loading scanner: line\n")
                && text.ends_with("...")
                && text.len() <= DIAGNOSTIC_BYTES)
        );
    }
}
