//! The GS1 mod-10 check digit against known-good keys.

use super::gs1_check_digit;

#[test]
fn gs1_check_digit_matches_known_keys() {
    // GTIN-14 and GLN both use the same mod-10 routine.
    assert_eq!(gs1_check_digit(&[0, 9, 5, 0, 6, 0, 0, 0, 1, 3, 4, 3, 5]), 2);
    assert_eq!(gs1_check_digit(&[4, 0, 1, 2, 3, 4, 5, 0, 0, 0, 0, 0]), 9);
}
