//! [`Icd`] — an ISO/IEC 6523 International Code Designator.

use serde::{Deserialize, Serialize};

use super::error::ActorIdentifierError;

/// The ICD of the Legal Entity Identifier scheme.
///
/// 🚨 **Read from copies of the ICD list, not from the Registration Authority's.**
/// ISO/IEC 6523-2:2025 makes the Registration Authority's list the register, and
/// its published list did not resolve on 2026-10-20. The code is the one three
/// sources agree on: the Commission's EAS code list, version 17.0, where the entry
/// is marked as coming from the ICD list; the PEPPOL BIS Billing 3.0 May 2026
/// release copy of the ICD list (243 codes, 0002 to 0248, no version stated); and
/// Table C.10 of EN 18219:2026. Check it against the Registration Authority's list
/// before relying on it past this build.
pub(super) const LEI: &str = "0199";

/// The ICD of the Data Universal Numbering System (D-U-N-S).
///
/// From the same three sources, with the same caveat, as [`LEI`].
pub(super) const DUNS: &str = "0060";

/// An ISO/IEC 6523 International Code Designator: the four digits that name the
/// registered identification scheme a clause 6.2 identifier is in.
///
/// **Only the shape is checked.** Whether a code is *registered* is the
/// Registration Authority's list, which this crate does not hold, so
/// `Icd::parse("9999")` succeeds whether or not that code was ever allocated.
/// What this type refuses is anything that cannot be an ICD at all.
///
/// Serialises as a bare string, and deserialization refuses what is not four
/// digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Icd(String);

impl Icd {
    /// Parse an ICD: exactly four ASCII digits.
    ///
    /// # Errors
    ///
    /// [`ActorIdentifierError::InvalidIcd`] for anything else.
    pub fn parse(s: &str) -> Result<Self, ActorIdentifierError> {
        if s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit()) {
            Ok(Self(s.to_owned()))
        } else {
            Err(ActorIdentifierError::InvalidIcd(s.to_owned()))
        }
    }

    /// The Legal Entity Identifier scheme, ICD `0199`.
    #[must_use]
    pub fn lei() -> Self {
        Self(LEI.to_owned())
    }

    /// The D-U-N-S scheme, ICD `0060`.
    #[must_use]
    pub fn duns() -> Self {
        Self(DUNS.to_owned())
    }

    /// The four digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for Icd {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Icd {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}
