//! Structure files (spec `ardent-schemas`): the stamp of a source file and the nodes of its outline, cached in
//! `.ardent/cache/files/<source path>/structure.yaml`.

use std::io;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::outline::Outline;
use crate::yaml::{self, ReadError};

/// The schema version of structure files that this Ardent reads and writes.
pub const SCHEMA: u32 = 2;

/// What a sync compares to decide whether a structure is current (decision-2): the size, time, and hash of the source
/// file, and the format plugin that built the outline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub size: u64,
    #[serde(with = "yaml::timestamp")]
    pub mtime: SystemTime,
    /// The full BLAKE3 hash of the source file, from [`content_hash`].
    pub hash: String,
    pub format: String,
    pub format_version: String,
    pub format_hash: String,
}

/// The stored form of the outline of one source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Structure {
    schema: u32,
    pub stamp: Stamp,
    #[serde(rename = "nodes")]
    pub outline: Outline,
}

impl Structure {
    pub fn new(stamp: Stamp, outline: Outline) -> Self {
        Self {
            schema: SCHEMA,
            stamp,
            outline,
        }
    }

    pub fn to_yaml(&self) -> io::Result<String> {
        yaml::to_string(self)
    }

    pub fn from_yaml(text: &str) -> Result<Self, ReadError> {
        yaml::read(text, SCHEMA)
    }
}

/// The content hash of a stamp: the full BLAKE3 hash of `source` as 64 lowercase hex digits.
pub fn content_hash(source: &[u8]) -> String {
    blake3::hash(source).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::ops::Range;
    use std::time::Duration;

    use jiff::Timestamp;

    use super::*;
    use crate::outline::ScannedNode;

    /// The Rust example of the spec `ardent-schemas`, and its structure file.
    const ENGINE_RS: &str = "pub struct Engine {
    files: Vec<String>,
}

impl Engine {
    pub fn run(&self) -> usize {
        self.files.len()
    }

    fn reset(&mut self) {
        self.files.clear();
    }
}
";
    const ENGINE_STRUCTURE: &str = "schema: 2
stamp:
  size: 188
  mtime: 2026-11-20T08:15:42.123456789Z
  hash: bc0644bb4c4483bf81dfa058498edb35826be58d2bd29f040f0607b59aee46a0
  format: rust
  format_version: 0.1.0
  format_hash: 657afc1b346ceeac0ae244c175c42108bbd218969077353426ade6eb6b5f6f52
nodes:
  - path: src/engine.rs
    label: src/engine.rs
    kind: file
    range: 0..188
    fingerprint: 174add0298423e83
  - path: src/engine.rs#impl Engine
    label: impl Engine
    kind: impl
    range: 47..187
    fingerprint: e756306dadb3fb00
  - path: src/engine.rs#impl Engine/fn run
    label: fn run
    kind: fn
    range: 65..124
    fingerprint: a33b27c3382d2785
  - path: src/engine.rs#impl Engine/fn reset
    label: fn reset
    kind: fn
    range: 130..185
    fingerprint: 0c3892e426b726a6
";

    fn node(kind: &str, label: &str, range: Range<usize>, labels: &[&str]) -> ScannedNode {
        let label_ranges = labels
            .iter()
            .filter_map(|word| {
                let start = range.start + ENGINE_RS.get(range.clone())?.find(word)?;
                Some(start..start + word.len())
            })
            .collect();
        ScannedNode {
            kind: kind.to_string(),
            label: label.to_string(),
            range,
            label_ranges,
            children: Vec::new(),
        }
    }

    fn engine_structure() -> Result<Structure, Box<dyn Error>> {
        let mut engine = node("impl", "impl Engine", 47..187, &["impl", "Engine"]);
        engine.children = vec![
            node("fn", "fn run", 65..124, &["fn", "run"]),
            node("fn", "fn reset", 130..185, &["fn", "reset"]),
        ];
        let stamp = Stamp {
            size: 188,
            mtime: SystemTime::try_from("2026-11-20T08:15:42.123456789Z".parse::<Timestamp>()?)?,
            hash: content_hash(ENGINE_RS.as_bytes()),
            format: "rust".to_string(),
            format_version: "0.1.0".to_string(),
            format_hash: "657afc1b346ceeac0ae244c175c42108bbd218969077353426ade6eb6b5f6f52"
                .to_string(),
        };
        let outline = Outline::build("src/engine.rs", ENGINE_RS.as_bytes(), &[engine]);
        Ok(Structure::new(stamp, outline))
    }

    #[test]
    fn writes_the_rust_example_of_the_spec() -> Result<(), Box<dyn Error>> {
        assert_eq!(engine_structure()?.to_yaml()?, ENGINE_STRUCTURE);
        Ok(())
    }

    #[test]
    fn reads_back_what_it_writes() -> Result<(), Box<dyn Error>> {
        let structure = engine_structure()?;
        assert_eq!(Structure::from_yaml(&structure.to_yaml()?)?, structure);
        Ok(())
    }

    #[test]
    fn round_trip_keeps_quoted_labels_and_old_times() -> Result<(), Box<dyn Error>> {
        let mut structure = engine_structure()?;
        structure.stamp.mtime = SystemTime::UNIX_EPOCH - Duration::new(10, 1);
        structure.outline.nodes[1].path =
            "src/engine.rs#Note: this/- 'item' \"x\" #tag".to_string();
        structure.outline.nodes[2].fingerprint = "1234567890123456".to_string();
        assert_eq!(Structure::from_yaml(&structure.to_yaml()?)?, structure);
        Ok(())
    }

    #[test]
    fn rejects_an_unknown_schema() {
        let text = ENGINE_STRUCTURE.replace("schema: 2", "schema: 7");
        assert!(matches!(
            Structure::from_yaml(&text),
            Err(ReadError::UnknownSchema(Some(7)))
        ));
        let text = ENGINE_STRUCTURE.replace("schema: 2\n", "");
        assert!(matches!(
            Structure::from_yaml(&text),
            Err(ReadError::UnknownSchema(None))
        ));
    }

    #[test]
    fn rejects_an_unknown_key() {
        let text = ENGINE_STRUCTURE.replacen("    kind: file", "    kindd: file", 1);
        let err = Structure::from_yaml(&text).err().map(|err| err.to_string());
        assert!(err.is_some_and(|message| message.contains("kindd")));
    }

    #[test]
    fn rejects_text_that_is_not_yaml() {
        assert!(matches!(
            Structure::from_yaml("nodes: ["),
            Err(ReadError::Syntax(_))
        ));
    }

    #[test]
    fn rejects_a_reversed_range() {
        let text = ENGINE_STRUCTURE.replace("range: 65..124", "range: 124..65");
        assert!(matches!(
            Structure::from_yaml(&text),
            Err(ReadError::Invalid(_))
        ));
    }
}
