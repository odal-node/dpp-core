//! [`CnCategory`] — the combined-nomenclature chapter or heading a discarded
//! product line is disclosed under.

use serde::{Deserialize, Serialize};

use super::error::CnCategoryError;

/// A combined-nomenclature **chapter** (2 digits) or **heading** (4 digits).
///
/// # Not [`CommodityCode`](crate::identifier::CommodityCode)
///
/// The two are different levels of the same nomenclature and must not be
/// substituted for one another. `CommodityCode` is a *product's own*
/// classification — 6, 8 or 10 digits — and answers "what is this thing". This
/// answers "which line of a disclosure does it belong on", and the applicable
/// act fixes the depth.
///
/// **Commission Implementing Regulation (EU) 2026/2, Art. 3:** the disclosure of
/// discarded unsold consumer products "shall be delimited based on the **first
/// two digits** of the relevant combined nomenclature (CN) codes set out in
/// Annex I to Regulation (EEC) No 2658/87. However, the products listed in Annex
/// II to this Regulation shall be delimited based on the **first four digits**".
///
/// So both depths are legitimate and which one is required depends on the
/// product. That test needs the Annex II list and lives in `dpp-rules`, not
/// here: this type refuses what is structurally malformed and makes no claim
/// about whether the depth is the right one — the same division of labour as
/// `CommodityCode` and [`Gtin`](crate::Gtin).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CnCategory(String);

impl CnCategory {
    /// Parse a CN chapter or heading.
    ///
    /// Surrounding whitespace is trimmed; separators are **not**. Compacting
    /// `"62 03"` would turn a mistyped value into a different, valid heading —
    /// the same reason `CommodityCode` refuses them.
    ///
    /// # Errors
    /// [`CnCategoryError::InvalidFormat`] unless the trimmed input is exactly 2
    /// or 4 ASCII digits.
    pub fn parse(s: &str) -> Result<Self, CnCategoryError> {
        let trimmed = s.trim();
        let valid_length = matches!(trimmed.len(), 2 | 4);
        if !valid_length || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return Err(CnCategoryError::InvalidFormat(s.to_owned()));
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The category as stored.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The CN chapter — the first two digits, whichever depth this is.
    ///
    /// A heading always extends a chapter, so this is the part two categories at
    /// different depths can be compared on.
    #[must_use]
    pub fn chapter(&self) -> &str {
        &self.0[..2]
    }

    /// Whether this is a 4-digit heading rather than a 2-digit chapter.
    #[must_use]
    pub fn is_heading(&self) -> bool {
        self.0.len() == 4
    }
}

impl std::fmt::Display for CnCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
