//! Unsold-goods destruction ban — ESPR Art. 25 / Annex VII scope.
//!
//! ## Annex VII scope test (cross-field)
//! Annex VII, "Consumer products of which the destruction by economic
//! operators is prohibited", is a commodity-code table with exactly two
//! headings, taken from the combined nomenclature (Council Regulation (EEC)
//! No 2658/87, Annex I, version in force 28 June 2024):
//!
//! - Apparel and clothing accessories: CN `4203`, `61`, `62`, `6504`, `6505`
//! - Footwear: CN `6401`–`6405`
//!
//! The codes are chapter- (2-digit) and heading-level (4-digit); a product's
//! own commodity code is 6, 8 or 10 digits, so the scope test is **prefix
//! matching**, never equality.

// ── Annex VII commodity-code prefixes ───────────────────────────────────────

/// Annex VII heading 1 — apparel and clothing accessories.
const APPAREL_AND_CLOTHING_ACCESSORIES_PREFIXES: &[&str] = &["4203", "61", "62", "6504", "6505"];

/// Annex VII heading 2 — footwear. Deliberately excludes `6406` (parts of
/// footwear), which Annex VII does not list.
const FOOTWEAR_PREFIXES: &[&str] = &["6401", "6402", "6403", "6404", "6405"];

/// Which of Annex VII's two headings a commodity code falls under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnexViiHeading {
    ApparelAndClothingAccessories,
    Footwear,
}

/// The Annex VII heading a commodity code falls under, if any. Prefix match
/// against the two headings above — `commodity_code` is expected to be a
/// bare 6/8/10 digit string (as produced by `CommodityCode::as_str()` in
/// `dpp-domain`).
#[must_use]
pub fn annex_vii_heading(commodity_code: &str) -> Option<AnnexViiHeading> {
    if APPAREL_AND_CLOTHING_ACCESSORIES_PREFIXES
        .iter()
        .any(|prefix| commodity_code.starts_with(prefix))
    {
        Some(AnnexViiHeading::ApparelAndClothingAccessories)
    } else if FOOTWEAR_PREFIXES
        .iter()
        .any(|prefix| commodity_code.starts_with(prefix))
    {
        Some(AnnexViiHeading::Footwear)
    } else {
        None
    }
}

/// Whether a commodity code falls within ESPR Annex VII's destruction-ban
/// scope at all (apparel & clothing accessories, or footwear).
#[must_use]
pub fn is_within_annex_vii_scope(commodity_code: &str) -> bool {
    annex_vii_heading(commodity_code).is_some()
}

#[cfg(test)]
mod tests;
