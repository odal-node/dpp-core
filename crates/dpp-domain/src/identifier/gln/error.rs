//! [`GlnError`] — why a GLN string was refused.

use thiserror::Error;

/// Error from constructing a [`Gln`](super::Gln).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GlnError {
    #[error("GLN must be exactly 13 ASCII digits, got '{0}'")]
    InvalidFormat(String),
    #[error("GLN check digit invalid for '{gln}': expected {expected}, got {actual}")]
    InvalidCheckDigit {
        gln: String,
        expected: u8,
        actual: u8,
    },
}
