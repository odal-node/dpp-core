//! [`CommodityCode`] — the product's customs tariff classification.
//!
//! **Where this comes from.** Commission Implementing Regulation (EU) 2026/1778
//! has the registry verify, "where relevant, the validity of the commodity code
//! of the product in relation to the permitted ranges for this product group"
//! (Art. 8(7)(d)), and store it as part of the registration data
//! (Art. 8(9)(b)). It is also what the registry's storage component holds for
//! products going under the customs procedure "release for free circulation"
//! (Art. 3(e)) — which is to say, it is the field customs authorities act on.
//!
//! # What is validated here, and what is not
//!
//! The code is a **structural** newtype: 6, 8 or 10 ASCII digits, matching the
//! three nested levels in use —
//!
//! - **6 digits** — Harmonised System subheading (WCO, global);
//! - **8 digits** — Combined Nomenclature, the EU's HS extension and the level
//!   most product legislation cites;
//! - **10 digits** — TARIC, the CN plus Union measures.
//!
//! Whether a *particular* code is the right one for a product, or falls inside
//! the range a product group permits, is not checkable here: the permitted
//! ranges live in the applicable delegated act and the registry applies them on
//! submission. This type refuses what is definitely malformed and makes no claim
//! about what is merely wrong — the same division of labour as [`crate::Gtin`],
//! which checks the check digit and not whether the product exists.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error from constructing a [`CommodityCode`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CommodityCodeError {
    /// Not 6, 8 or 10 ASCII digits.
    #[error("commodity code must be 6 (HS), 8 (CN) or 10 (TARIC) ASCII digits, got '{0}'")]
    InvalidFormat(String),
}

/// A validated customs tariff classification: HS-6, CN-8 or TARIC-10.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommodityCode(String);

impl CommodityCode {
    /// Parse a commodity code, accepting the HS, CN and TARIC lengths.
    ///
    /// Surrounding whitespace is trimmed; separators are **not**. A code written
    /// `"8507 60 00"` is rejected rather than silently compacted, because
    /// stripping characters to make a value parse is how a typo becomes a
    /// different, valid tariff heading.
    pub fn parse(s: &str) -> Result<Self, CommodityCodeError> {
        let trimmed = s.trim();
        let valid_length = matches!(trimmed.len(), 6 | 8 | 10);
        if !valid_length || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return Err(CommodityCodeError::InvalidFormat(s.to_owned()));
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The code as stored.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The HS-6 subheading this code sits under — its own first six digits.
    ///
    /// Every CN-8 and TARIC-10 code extends an HS-6 subheading, so this is the
    /// globally comparable part of any of the three.
    #[must_use]
    pub fn hs_subheading(&self) -> &str {
        &self.0[..6]
    }
}

impl std::fmt::Display for CommodityCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_tariff_levels_parse() {
        // HS-6 subheading, CN-8, TARIC-10 — lithium-ion accumulators.
        for code in ["850760", "85076000", "8507600090"] {
            let parsed = CommodityCode::parse(code).expect("must parse");
            assert_eq!(parsed.as_str(), code);
            assert_eq!(parsed.hs_subheading(), "850760");
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(
            CommodityCode::parse("  85076000 ").unwrap().as_str(),
            "85076000"
        );
    }

    /// Separators are refused rather than stripped: compacting `"8507 60 00"`
    /// would turn a mistyped code into a different valid tariff heading.
    #[test]
    fn separators_are_refused_not_stripped() {
        for code in ["8507 60 00", "8507.60.00", "8507-60-00"] {
            assert!(
                CommodityCode::parse(code).is_err(),
                "{code} must be refused rather than compacted"
            );
        }
    }

    #[test]
    fn wrong_lengths_are_refused() {
        // 4 (heading), 7, 9 and 12 digits are not classification levels.
        for code in ["8507", "8507600", "850760009", "850760009012", ""] {
            assert!(
                CommodityCode::parse(code).is_err(),
                "{code} must be refused"
            );
        }
    }

    #[test]
    fn round_trips_as_a_bare_json_string() {
        let code = CommodityCode::parse("85076000").unwrap();
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"85076000\"");
        assert_eq!(
            serde_json::from_str::<CommodityCode>(&json).unwrap(),
            code,
            "serde(transparent): the wire form is the code itself"
        );
    }
}
