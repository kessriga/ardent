//! Reuse the open document's plugin results across syncs that supply fresh plugin handles.
use std::sync::Arc;

use ardent_core::highlight::Highlight;
use ardent_core::properties::NodeProperties;
use ardent_core::structure::Stamp;
use ardent_format::FormatPlugin;

#[derive(Default)]
pub(super) struct DocumentPlugins {
    pub(super) formats: Vec<FormatPlugin>,
    cached: Option<Cached>,
}

struct Cached {
    source_hash: String,
    format_hash: String,
    outline_len: usize,
    details: Details,
}

#[derive(Clone, Default)]
pub(super) struct Details {
    pub(super) highlights: Arc<[Highlight]>,
    pub(super) properties: Arc<NodeProperties>,
}

impl DocumentPlugins {
    pub(super) fn new(formats: Vec<FormatPlugin>) -> Self {
        Self {
            formats,
            cached: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.cached = None;
    }

    pub(super) fn details(&mut self, stamp: &Stamp, bytes: &[u8], outline_len: usize) -> Details {
        let Some(plugin) = self
            .formats
            .iter_mut()
            .find(|plugin| plugin.name == stamp.format && plugin.format_hash == stamp.format_hash)
        else {
            return Details::default();
        };
        if let Some(cached) = &self.cached
            && cached.source_hash == stamp.hash
            && cached.format_hash == stamp.format_hash
            && cached.outline_len == outline_len
        {
            return cached.details.clone();
        }
        let highlights = plugin.highlight(bytes);
        let properties = plugin.properties(bytes, outline_len);
        let reusable = highlights.is_ok() && properties.is_ok();
        let details = Details {
            highlights: highlights.unwrap_or_default().into(),
            properties: Arc::new(properties.unwrap_or_default()),
        };
        if reusable {
            self.cached = Some(Cached {
                source_hash: stamp.hash.clone(),
                format_hash: stamp.format_hash.clone(),
                outline_len,
                details: details.clone(),
            });
        }
        details
    }
}
