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

/// Whether a declared `UnsoldGoodsReport.product_category` word is
/// consistent with the Annex VII heading a passport's `commodity_code`
/// actually falls under.
///
/// `"accessories"` is part of heading 1 (apparel and clothing accessories),
/// not a peer of `"apparel"` — Annex VII has two headings, not three, so
/// both words are consistent with that one heading. `"home-textile"` and
/// `"other"` correspond to **no** Annex VII heading (Annex VII does not
/// cover home textiles at all), so they contradict any in-scope commodity
/// code — there is no heading they could ever match.
#[must_use]
pub fn product_category_matches_heading(product_category: &str, heading: AnnexViiHeading) -> bool {
    matches!(
        (product_category, heading),
        (
            "apparel" | "accessories",
            AnnexViiHeading::ApparelAndClothingAccessories
        ) | ("footwear", AnnexViiHeading::Footwear)
    )
}

#[cfg(test)]
mod tests {
    use super::{
        AnnexViiHeading, annex_vii_heading, is_within_annex_vii_scope,
        product_category_matches_heading,
    };

    #[test]
    fn apparel_heading_prefixes_are_in_scope() {
        assert!(is_within_annex_vii_scope("420321")); // leather apparel article
        assert!(is_within_annex_vii_scope("610910")); // chapter 61, t-shirts
        assert!(is_within_annex_vii_scope("620342")); // chapter 62, trousers
        assert!(is_within_annex_vii_scope("650400")); // hats
        assert!(is_within_annex_vii_scope("650500")); // headgear
    }

    #[test]
    fn footwear_heading_prefixes_are_in_scope() {
        assert!(is_within_annex_vii_scope("64011000"));
        assert!(is_within_annex_vii_scope("64029900"));
        assert!(is_within_annex_vii_scope("64031900"));
        assert!(is_within_annex_vii_scope("64041100"));
        assert!(is_within_annex_vii_scope("64051000"));
    }

    #[test]
    fn footwear_parts_are_out_of_scope() {
        // 6406 (parts of footwear) is deliberately not one of Annex VII's
        // five footwear headings.
        assert!(!is_within_annex_vii_scope("64062000"));
    }

    #[test]
    fn unrelated_chapter_is_out_of_scope() {
        assert!(!is_within_annex_vii_scope("851712")); // smartphones (HS 8517)
    }

    #[test]
    fn annex_vii_heading_reports_which_heading_matched() {
        assert_eq!(
            annex_vii_heading("620342"),
            Some(AnnexViiHeading::ApparelAndClothingAccessories)
        );
        assert_eq!(
            annex_vii_heading("64011000"),
            Some(AnnexViiHeading::Footwear)
        );
        assert_eq!(annex_vii_heading("851712"), None);
    }

    #[test]
    fn apparel_and_accessories_both_match_the_one_apparel_heading() {
        assert!(product_category_matches_heading(
            "apparel",
            AnnexViiHeading::ApparelAndClothingAccessories
        ));
        assert!(product_category_matches_heading(
            "accessories",
            AnnexViiHeading::ApparelAndClothingAccessories
        ));
    }

    #[test]
    fn footwear_matches_only_the_footwear_heading() {
        assert!(product_category_matches_heading(
            "footwear",
            AnnexViiHeading::Footwear
        ));
        assert!(!product_category_matches_heading(
            "footwear",
            AnnexViiHeading::ApparelAndClothingAccessories
        ));
    }

    #[test]
    fn crossed_categories_do_not_match() {
        assert!(!product_category_matches_heading(
            "apparel",
            AnnexViiHeading::Footwear
        ));
        assert!(!product_category_matches_heading(
            "accessories",
            AnnexViiHeading::Footwear
        ));
    }

    #[test]
    fn home_textile_and_other_match_no_heading() {
        // Annex VII has no home-textile heading at all, so these two words
        // can never be consistent with an in-scope commodity code.
        for category in ["home-textile", "other"] {
            assert!(!product_category_matches_heading(
                category,
                AnnexViiHeading::ApparelAndClothingAccessories
            ));
            assert!(!product_category_matches_heading(
                category,
                AnnexViiHeading::Footwear
            ));
        }
    }
}
