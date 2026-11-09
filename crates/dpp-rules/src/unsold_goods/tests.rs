//! Tests for the Art. 3 category a CN code is disclosed under.

use super::disclosure::category_of_cn_code;

/// Heading 8516 is listed in Annex II, so Art. 3 requires four digits.
#[test]
fn an_annex_ii_heading_is_filed_at_four_digits() {
    assert_eq!(category_of_cn_code("85161000"), Some("8516"));
    assert_eq!(category_of_cn_code("420321"), Some("4203"));
    assert_eq!(category_of_cn_code("9503000000"), Some("9503"));
}

/// Heading 6203 is not listed in Annex II, so the chapter is the category.
#[test]
fn a_code_outside_annex_ii_is_filed_at_its_chapter() {
    assert_eq!(category_of_cn_code("62034200"), Some("62"));
    assert_eq!(category_of_cn_code("640399"), Some("64"));
}

/// A chapter holding an Annex II heading still files its other headings at two
/// digits: 8517 is listed, 8514 is not.
#[test]
fn a_heading_beside_an_annex_ii_heading_is_still_filed_at_its_chapter() {
    assert_eq!(category_of_cn_code("85141000"), Some("85"));
}

#[test]
fn a_code_that_is_not_six_eight_or_ten_digits_has_no_category() {
    for unreadable in [
        "",
        "62",
        "6203",
        "62034",
        "620342000",
        "6203420x",
        "abcdefgh",
    ] {
        assert_eq!(category_of_cn_code(unreadable), None, "{unreadable:?}");
    }
}
