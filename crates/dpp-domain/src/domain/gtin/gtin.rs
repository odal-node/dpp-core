//! [`Gtin`] — a validated GS1 GTIN-14, and the error refusing an invalid one.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::check_digit::{Gs1KeyCheck, check_gs1_key};

/// Error from constructing a [`Gtin`].
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

/// A validated GS1 GTIN-14 (14-digit trade item number, GS1 mod-10 check digit verified).
///
/// Construct via [`Gtin::parse`]. Serialises/deserialises as a bare string;
/// deserialization rejects invalid GTINs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Gtin(String);

impl Gtin {
    /// Parse and validate a GTIN, normalising the shorter GS1 forms to GTIN-14.
    ///
    /// Accepts 8, 12, 13 or 14 ASCII digits — the four lengths GS1 defines —
    /// and stores the canonical 14-digit form, so [`as_str`](Gtin::as_str)
    /// always returns 14 digits and downstream comparisons stay exact.
    ///
    /// # Why padding is safe
    ///
    /// GS1 defines GTIN-8/12/13 as right-aligned within a 14-digit field,
    /// zero-filled on the left: `03801234567898` and `3801234567898` are the
    /// same trade item. The check digit is computed from the right with
    /// alternating weights, so a leading zero contributes nothing and the
    /// padded form validates identically. Normalising here rather than in every
    /// caller is what keeps a retail EAN-13 — the form an operator actually has
    /// in their product data — from reading as malformed input.
    ///
    /// Lengths GS1 does not define (9, 10, 11, 15+) are still refused: padding
    /// one would invent an identifier rather than restate a known one. A wrong
    /// check digit and a non-digit character are refused as before.
    pub fn parse(s: &str) -> Result<Self, GtinError> {
        if !matches!(s.len(), 8 | 12 | 13 | 14) || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(GtinError::InvalidFormat(s.to_owned()));
        }
        let canonical = format!("{s:0>14}");
        match check_gs1_key(&canonical, 14) {
            Ok(()) => Ok(Self(canonical)),
            Err(Gs1KeyCheck::InvalidFormat) => Err(GtinError::InvalidFormat(s.to_owned())),
            Err(Gs1KeyCheck::InvalidCheckDigit { expected, actual }) => {
                Err(GtinError::InvalidCheckDigit {
                    gtin: s.to_owned(),
                    expected,
                    actual,
                })
            }
        }
    }

    /// The canonical 14-digit form, whatever length was parsed.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Gtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Compares against the **canonical 14-digit** form, not the string that was
/// parsed: `Gtin::parse("3801234567898")` equals `"03801234567898"`, not
/// `"3801234567898"`. Compare two `Gtin`s where the input spelling is unknown.
impl PartialEq<str> for Gtin {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl Serialize for Gtin {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Gtin {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}
