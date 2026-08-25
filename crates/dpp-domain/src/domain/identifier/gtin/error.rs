//! [`GtinError`] — why a GTIN string was refused.

use thiserror::Error;

/// Error from constructing a [`Gtin`](super::Gtin).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GtinError {
    #[error("GTIN must be 8, 12, 13 or 14 ASCII digits, got '{0}'")]
    InvalidFormat(String),
    #[error("GTIN check digit invalid for '{gtin}': expected {expected}, got {actual}")]
    InvalidCheckDigit {
        gtin: String,
        expected: u8,
        actual: u8,
    },
}
