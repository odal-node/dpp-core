//! Validated GS1 newtypes (`Gtin`, `Gln`) with GS1 modulo-10 check-digit
//! verification, plus the shared check-digit primitive.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Compute the GS1 modulo-10 check digit for the *data* portion of a GS1 key.
///
/// Shared by GTIN-14, GLN-13 and other fixed-length GS1 numeric keys. The
/// rightmost data digit carries weight 3, then alternating 1,3,… leftward
/// (the canonical GS1 rule). `data_digits` holds values 0–9 and excludes the
/// trailing check digit.
#[must_use]
pub fn gs1_check_digit(data_digits: &[u8]) -> u8 {
    let sum: u32 = data_digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| u32::from(d) * if i % 2 == 0 { 3 } else { 1 })
        .sum();
    ((10 - (sum % 10)) % 10) as u8
}

/// Error from constructing a [`Gtin`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GtinError {
    #[error("GTIN must be exactly 14 ASCII digits, got '{0}'")]
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
    /// Parse and validate a GTIN-14 string.
    ///
    /// Accepts exactly 14 ASCII digits with a correct GS1 modulo-10 check digit
    /// (alternating weights 3,1,3,1,… from left). Returns `Err` for wrong length,
    /// non-digit characters, or a bad check digit.
    pub fn parse(s: &str) -> Result<Self, GtinError> {
        if s.len() != 14 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(GtinError::InvalidFormat(s.to_owned()));
        }
        let digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
        let expected = gs1_check_digit(&digits[..13]);
        if digits[13] != expected {
            return Err(GtinError::InvalidCheckDigit {
                gtin: s.to_owned(),
                expected,
                actual: digits[13],
            });
        }
        Ok(Self(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Gtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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

/// Error from constructing a [`Gln`].
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

/// A validated GS1 GLN (13-digit Global Location Number) with its GS1 mod-10
/// check digit verified — the same algorithm as [`Gtin`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gln(String);

impl Gln {
    /// Parse a GLN: exactly 13 ASCII digits with a correct GS1 modulo-10 check
    /// digit. Returns `Err` for wrong length, non-digits, or a bad check digit.
    pub fn parse(s: &str) -> Result<Self, GlnError> {
        if s.len() != 13 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(GlnError::InvalidFormat(s.to_owned()));
        }
        let digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
        let expected = gs1_check_digit(&digits[..12]);
        if digits[12] != expected {
            return Err(GlnError::InvalidCheckDigit {
                gln: s.to_owned(),
                expected,
                actual: digits[12],
            });
        }
        Ok(Self(s.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 09506000134352 — verified valid GTIN-14 used throughout the test suite.
    const VALID: &str = "09506000134352";

    #[test]
    fn valid_gtin_parses() {
        assert!(Gtin::parse(VALID).is_ok());
    }

    #[test]
    fn wrong_length_rejected() {
        assert!(matches!(
            Gtin::parse("095060001343"),
            Err(GtinError::InvalidFormat(_))
        ));
    }

    #[test]
    fn non_digits_rejected() {
        assert!(matches!(
            Gtin::parse("0950600013435X"),
            Err(GtinError::InvalidFormat(_))
        ));
    }

    #[test]
    fn bad_check_digit_rejected() {
        // Last digit changed from 2 → 1: wrong check digit.
        assert!(matches!(
            Gtin::parse("09506000134351"),
            Err(GtinError::InvalidCheckDigit { .. })
        ));
    }

    #[test]
    fn display_equals_inner_string() {
        let g = Gtin::parse(VALID).unwrap();
        assert_eq!(g.to_string(), VALID);
    }

    #[test]
    fn partial_eq_str() {
        let g = Gtin::parse(VALID).unwrap();
        assert_eq!(g, *VALID); // PartialEq<str>: gtin == *str_ref
        assert_eq!(g.as_str(), VALID); // as_str() for direct &str comparison
    }

    #[test]
    fn serde_round_trip() {
        let g = Gtin::parse(VALID).unwrap();
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(json, format!("\"{}\"", VALID));
        let back: Gtin = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn invalid_gtin_fails_deserialization() {
        // Check digit wrong.
        let result = serde_json::from_str::<Gtin>("\"09506000134351\"");
        assert!(result.is_err());
    }

    #[test]
    fn prepend_zero_to_valid_ean13_gives_valid_gtin14() {
        // 1234567890128 is a valid EAN-13; prepending 0 gives a valid GTIN-14.
        assert!(Gtin::parse("01234567890128").is_ok());
    }

    // ── GLN ──────────────────────────────────────────────────────────────────

    #[test]
    fn valid_gln_parses() {
        // 4012345000009 — GS1 mod-10 check digit verified.
        assert!(Gln::parse("4012345000009").is_ok());
    }

    #[test]
    fn gln_wrong_length_rejected() {
        assert!(matches!(
            Gln::parse("401234500000"),
            Err(GlnError::InvalidFormat(_))
        ));
    }

    #[test]
    fn gln_non_digits_rejected() {
        assert!(matches!(
            Gln::parse("401234500000X"),
            Err(GlnError::InvalidFormat(_))
        ));
    }

    #[test]
    fn gln_bad_check_digit_rejected() {
        // 4000001000002 has a wrong check digit (should be …5).
        assert!(matches!(
            Gln::parse("4000001000002"),
            Err(GlnError::InvalidCheckDigit { .. })
        ));
        assert!(Gln::parse("4000001000005").is_ok());
    }

    #[test]
    fn gs1_check_digit_matches_known_keys() {
        // GTIN-14 and GLN both use the same mod-10 routine.
        assert_eq!(gs1_check_digit(&[0, 9, 5, 0, 6, 0, 0, 0, 1, 3, 4, 3, 5]), 2);
        assert_eq!(gs1_check_digit(&[4, 0, 1, 2, 3, 4, 5, 0, 0, 0, 0, 0]), 9);
    }
}
