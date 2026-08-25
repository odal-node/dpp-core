//! CN category parsing: chapter, heading, and what is refused.

use super::CnCategory;

#[test]
fn a_two_digit_chapter_parses() {
    assert_eq!(CnCategory::parse("62").unwrap().as_str(), "62");
}

#[test]
fn a_four_digit_heading_parses() {
    assert_eq!(CnCategory::parse("6203").unwrap().as_str(), "6203");
}

#[test]
fn a_full_commodity_code_is_not_a_category() {
    assert!(CnCategory::parse("62034231").is_err());
}
