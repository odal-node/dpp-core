//! Tests of the `annex_vii` module.

use super::super::{AnnexViiHeading, annex_vii_heading, is_within_annex_vii_scope};

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
