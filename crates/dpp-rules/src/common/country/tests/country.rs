//! Tests of the `country` module.

use super::super::*;

#[test]
fn country_code_rules() {
    assert!(country_code_valid("DE"));
    assert!(!country_code_valid("de"));
    assert!(!country_code_valid("DEU"));
}

#[test]
fn assigned_codes_accepted() {
    for c in ["DE", "FR", "US", "MK", "SS", "AD", "ZW"] {
        assert!(country_code_valid(c), "{c} is an assigned ISO 3166-1 code");
    }
}

#[test]
fn well_formed_but_unassigned_codes_rejected() {
    // Correct shape, not officially assigned ⇒ must be rejected.
    for c in ["XX", "QZ", "AA", "ZZ", "OO"] {
        assert!(!country_code_valid(c), "{c} is not an assigned code");
    }
    // `EU`/`UK` are reserved, not officially assigned.
    assert!(!country_code_valid("EU"));
    assert!(!country_code_valid("UK"));
}

#[test]
fn table_is_sorted_for_binary_search() {
    assert!(ISO_3166_1_ALPHA2.windows(2).all(|w| w[0] < w[1]));
}
