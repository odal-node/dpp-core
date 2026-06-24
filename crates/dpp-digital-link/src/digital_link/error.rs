//! Error type for GS1 Digital Link parsing.

use dpp_domain::GtinError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DigitalLinkError {
    #[error("URI is missing the '/01/' GTIN segment")]
    MissingGtin,
    #[error("GTIN must be 8, 12, 13, or 14 digits, got '{0}'")]
    InvalidGtin(String),
    #[error("GTIN check digit invalid for '{gtin}': expected {expected}, got {actual}")]
    InvalidGtinCheckDigit {
        gtin: String,
        expected: u32,
        actual: u32,
    },
    #[error("URI scheme must be 'https', got '{0}'")]
    InvalidScheme(String),
    #[error("Unknown Application Identifier '{0}' in URI path")]
    UnknownApplicationIdentifier(String),
    #[error(
        "Qualifiers out of canonical order: '{after}' (order {after_ord}) must not follow '{before}' (order {before_ord})"
    )]
    QualifiersOutOfOrder {
        before: String,
        before_ord: u8,
        after: String,
        after_ord: u8,
    },
}

impl From<GtinError> for DigitalLinkError {
    fn from(e: GtinError) -> Self {
        match e {
            GtinError::InvalidFormat(s) => DigitalLinkError::InvalidGtin(s),
            GtinError::InvalidCheckDigit {
                gtin,
                expected,
                actual,
            } => DigitalLinkError::InvalidGtinCheckDigit {
                gtin,
                expected: u32::from(expected),
                actual: u32::from(actual),
            },
            // Forward-compat: map any future GtinError variant onto the generic
            // invalid-GTIN error, preserving its message.
            other => DigitalLinkError::InvalidGtin(other.to_string()),
        }
    }
}
