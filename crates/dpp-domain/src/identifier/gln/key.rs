//! [`Gln`] — a validated GS1 Global Location Number.

use serde::{Deserialize, Serialize};

use super::super::check_digit::{Gs1KeyCheck, check_gs1_key};
use super::error::GlnError;

/// A validated GS1 GLN (13-digit Global Location Number) with its GS1 mod-10
/// check digit verified — the same algorithm as [`Gtin`](crate::identifier::Gtin).
///
/// Serialises as a bare string, and deserialization refuses an invalid GLN, as
/// [`Gtin`](crate::identifier::Gtin) does. The typed EN 18219 clause 6 identifier
/// carries one, and is the first thing that needs it written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gln(String);

impl Gln {
    /// Parse a GLN: exactly 13 ASCII digits with a correct GS1 modulo-10 check
    /// digit. Returns `Err` for wrong length, non-digits, or a bad check digit.
    pub fn parse(s: &str) -> Result<Self, GlnError> {
        match check_gs1_key(s, 13) {
            Ok(()) => Ok(Self(s.to_owned())),
            Err(Gs1KeyCheck::InvalidFormat) => Err(GlnError::InvalidFormat(s.to_owned())),
            Err(Gs1KeyCheck::InvalidCheckDigit { expected, actual }) => {
                Err(GlnError::InvalidCheckDigit {
                    gln: s.to_owned(),
                    expected,
                    actual,
                })
            }
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Gln {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Gln {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}
