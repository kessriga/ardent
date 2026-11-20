//! The YAML form that every file in `.ardent/` shares (spec `ardent-schemas`): block style with keys in the order of
//! the spec, a leading `schema` version, unknown keys rejected, and ranges and times as one-line strings.

use std::fmt;
use std::io;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Why Ardent could not read a YAML file of `.ardent/`.
#[derive(Debug)]
pub enum ReadError {
    /// The text is not YAML, or its `schema` is not an integer.
    Syntax(String),
    /// The file has no `schema` key (`None`) or a version that this Ardent does not know.
    UnknownSchema(Option<u32>),
    /// The file has the known schema, but a key that the schema does not define or a value of the wrong type.
    Invalid(String),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(message) | Self::Invalid(message) => f.write_str(message),
            Self::UnknownSchema(None) => f.write_str("the file has no `schema` key"),
            Self::UnknownSchema(Some(version)) => write!(f, "unknown schema {version}"),
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads `text` as a `T` when its `schema` is `schema`. The version is checked first, so a file from a newer Ardent
/// reports its version and not the first key that this version does not know.
pub fn read<T: DeserializeOwned>(text: &str, schema: u32) -> Result<T, ReadError> {
    #[derive(Deserialize)]
    struct Header {
        schema: Option<u32>,
    }

    let header: Header =
        serde_saphyr::from_str(text).map_err(|err| ReadError::Syntax(err.to_string()))?;
    if header.schema != Some(schema) {
        return Err(ReadError::UnknownSchema(header.schema));
    }
    serde_saphyr::from_str(text).map_err(|err| ReadError::Invalid(err.to_string()))
}

/// Writes `value` in the block style of the spec examples: lists indented under their key, and quotes only where YAML
/// needs them. A value that YAML cannot hold, such as a time outside years 0 to 9999, fails with `InvalidData`.
pub fn to_string<T: Serialize>(value: &T) -> io::Result<String> {
    let options = serde_saphyr::ser_options! {
        compact_list_indent: false,
        yaml_12: true,
        no_lang_directive: true,
        prefer_block_scalars: false,
    };
    serde_saphyr::to_string_with_options(value, options)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

/// A byte range or line range as the string `start..end`, half-open as in Rust.
pub(crate) mod range {
    use std::ops::Range;

    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        range: &Range<usize>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{}..{}", range.start, range.end))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Range<usize>, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.split_once("..")
            .and_then(|(start, end)| Some(start.parse().ok()?..end.parse().ok()?))
            .filter(|range| range.start <= range.end)
            .ok_or_else(|| D::Error::custom(format!("{text:?} is not a range start..end")))
    }
}

/// A time as an RFC 3339 UTC timestamp with nine fraction digits, such as `2026-11-20T08:15:42.123456789Z`.
pub(crate) mod timestamp {
    use std::time::SystemTime;

    use serde::de::Error as _;
    use serde::ser::Error as _;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error> {
        let timestamp = jiff::Timestamp::try_from(*time).map_err(S::Error::custom)?;
        serializer.collect_str(&format_args!("{timestamp:.9}"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SystemTime, D::Error> {
        let text = String::deserialize(deserializer)?;
        let timestamp: jiff::Timestamp = text.parse().map_err(D::Error::custom)?;
        SystemTime::try_from(timestamp).map_err(D::Error::custom)
    }
}
