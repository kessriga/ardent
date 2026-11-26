//! Reuse the open document's spans across syncs that supply fresh plugin handles.
use std::sync::Arc;

use ardent_core::highlight::Highlight;
use ardent_core::structure::Stamp;
use ardent_format::FormatPlugin;

#[derive(Default)]
pub(super) struct Highlighter {
    pub(super) formats: Vec<FormatPlugin>,
    cached: Option<Cached>,
}

struct Cached {
    source_hash: String,
    format_hash: String,
    spans: Arc<[Highlight]>,
}

impl Highlighter {
    pub(super) fn new(formats: Vec<FormatPlugin>) -> Self {
        Self {
            formats,
            cached: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.cached = None;
    }

    pub(super) fn spans(&mut self, stamp: &Stamp, bytes: &[u8]) -> Arc<[Highlight]> {
        let Some(plugin) = self
            .formats
            .iter_mut()
            .find(|plugin| plugin.name == stamp.format && plugin.format_hash == stamp.format_hash)
        else {
            return Arc::default();
        };
        if let Some(cached) = &self.cached
            && cached.source_hash == stamp.hash
            && cached.format_hash == stamp.format_hash
        {
            return Arc::clone(&cached.spans);
        }
        let Ok(spans) = plugin.highlight(bytes) else {
            return Arc::default();
        };
        let spans = Arc::from(spans);
        self.cached = Some(Cached {
            source_hash: stamp.hash.clone(),
            format_hash: stamp.format_hash.clone(),
            spans: Arc::clone(&spans),
        });
        spans
    }
}
