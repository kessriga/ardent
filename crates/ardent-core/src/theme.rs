//! Foreground overrides in a user-installed theme.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::highlight::Category;
use crate::yaml::{self, ReadError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidRgb(String);

impl fmt::Display for InvalidRgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected a color of the form #RRGGBB, found {:?}",
            self.0
        )
    }
}

impl Error for InvalidRgb {}

impl TryFrom<String> for Rgb {
    type Error = InvalidRgb;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let digits = value
            .strip_prefix('#')
            .filter(|s| s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit()));
        let number = digits
            .and_then(|s| u32::from_str_radix(s, 16).ok())
            .ok_or(InvalidRgb(value))?;
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
    fn invalid_rgb_retains_the_input_in_a_typed_error() {
        let error = Rgb::try_from("red".to_owned()).expect_err("invalid RGB");
        assert_eq!(error, InvalidRgb("red".to_owned()));
        assert!(error.to_string().contains("#RRGGBB"));
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
