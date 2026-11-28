//! Foreground overrides in a user-installed theme.

use serde::Deserialize;
use std::collections::BTreeMap;

use crate::highlight::Category;
use crate::yaml::{self, ReadError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Rgb(pub u8, pub u8, pub u8);

impl TryFrom<String> for Rgb {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let digits = value
            .strip_prefix('#')
            .filter(|s| s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()));
        let number = digits
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .ok_or_else(|| format!("expected a color of the form #RRGGBB, found {value:?}"))?;
        let [_, red, green, blue] = number.to_be_bytes();
        Ok(Self(red, green, blue))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    #[serde(rename = "schema")]
    _schema: u32,
    #[serde(default)]
    pub source: BTreeMap<Category, Rgb>,
    #[serde(default)]
    pub outline: BTreeMap<String, Rgb>,
}

impl Theme {
    pub fn from_yaml(text: &str) -> Result<Self, ReadError> {
        yaml::read(text, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_all_categories_and_plugin_defined_kinds() -> Result<(), ReadError> {
        let theme = Theme::from_yaml(
            "source: {keyword: '#123456', string_literal: '#234567', comment: '#345678', heading: '#456789', code: '#56789a'}\noutline: {widget: '#abCDef'}\nschema: 1",
        )?;
        assert_eq!(theme.source.len(), 5);
        assert_eq!(
            theme.source[&Category::StringLiteral],
            Rgb(0x23, 0x45, 0x67)
        );
        assert_eq!(theme.outline["widget"], Rgb(0xab, 0xcd, 0xef));
        Ok(())
    }

    #[test]
    fn rejects_unknown_schema_keys_and_malformed_colors() {
        for text in [
            "source: {}",
            "schema: 2",
            "schema: 1\ncolors: {}",
            "schema: 1\nsource: {typo: '#123456'}",
            "schema: 1\nsource: {comment: '#GG1020'}",
            "schema: 1\noutline: {fn: '#+12345'}",
            "schema: 1\nsource: {code: '123456'}",
        ] {
            assert!(Theme::from_yaml(text).is_err(), "{text}");
        }
    }
}
