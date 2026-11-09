//! The Annex II heading list agrees with the Official Journal text.
//!
//! Commission Implementing Regulation (EU) 2026/2 Art. 3 has a disclosure filed
//! under the first two digits of a CN code, "However, the products listed in
//! Annex II … shall be delimited based on the first four digits". Which depth a
//! product is filed at therefore depends on this list, and one heading too many
//! files a product at four digits that the act has at two.
//!
//! The expected list below was transcribed **independently** from the first
//! column of the Annex II table in the EUR-Lex rendering of OJ L, 10.2.2026
//! (`https://eur-lex.europa.eu/eli/reg_impl/2026/2/oj/eng`, read 9 November 2026,
//! 44 rows), not from `dpp_rules`. A test that compared the list with itself
//! would agree by construction. Two copies that must be edited together are the
//! point: a change to either is a decision, not a slip.
//!
//! # `8527`
//!
//! The list carried `8527` for a while. It is not a heading of Annex II: the string
//! appears in the table once, inside the description of 8517 ("other than
//! transmission or reception apparatus of heading 8443, 8525, 8527 or 8528"), and
//! the Annex has 8528 but no 8527.

use dpp_rules::unsold_goods::disclosure::{annex_ii_headings_in_chapter, is_annex_ii_heading};

/// First column of the Annex II table, in the order the act prints it.
const OJ_ANNEX_II: [&str; 44] = [
    "3401", "3402", "4011", "4202", "4203", "4303", "4818", "6301", "6302", "6303", "6304", "6306",
    "6307", "8415", "8418", "8421", "8422", "8423", "8443", "8450", "8467", "8471", "8506", "8507",
    "8508", "8509", "8510", "8513", "8516", "8517", "8518", "8519", "8521", "8523", "8524", "8528",
    "8539", "9006", "9401", "9403", "9404", "9503", "9504", "9619",
];

/// Every four-digit heading the crate treats as Annex II, found through its public
/// API, so the test does not depend on how the list is stored.
fn headings_in_the_crate() -> Vec<String> {
    (0..=9999)
        .map(|n| format!("{n:04}"))
        .filter(|heading| is_annex_ii_heading(heading))
        .collect()
}

#[test]
fn annex_ii_lists_exactly_the_forty_four_headings_of_the_official_journal() {
    let ours = headings_in_the_crate();
    let theirs: Vec<String> = OJ_ANNEX_II.iter().map(|h| (*h).to_owned()).collect();

    let extra: Vec<&String> = ours.iter().filter(|h| !theirs.contains(h)).collect();
    let missing: Vec<&String> = theirs.iter().filter(|h| !ours.contains(h)).collect();

    assert!(
        extra.is_empty() && missing.is_empty(),
        "Annex II drifted from the Official Journal text: in the crate but not in the act \
         {extra:?}; in the act but not in the crate {missing:?}"
    );
}

/// A heading named only inside another row's description is not itself listed.
#[test]
fn a_heading_named_only_in_a_description_is_not_in_annex_ii() {
    assert!(!is_annex_ii_heading("8527"));
    assert!(!annex_ii_headings_in_chapter("85").contains(&"8527"));
}
