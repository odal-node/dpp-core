//! GLN parsing: length, digits, and the shared check digit.

use super::{Gln, GlnError};

#[test]
fn valid_gln_parses() {
    // 4012345000009 — GS1 mod-10 check digit verified.
    assert!(Gln::parse("4012345000009").is_ok());
}

#[test]
fn gln_wrong_length_rejected() {
    assert!(matches!(
        Gln::parse("401234500000"),
        Err(GlnError::InvalidFormat(_))
    ));
}

#[test]
fn gln_non_digits_rejected() {
    assert!(matches!(
        Gln::parse("401234500000X"),
        Err(GlnError::InvalidFormat(_))
    ));
}

#[test]
fn gln_bad_check_digit_rejected() {
    // 4000001000002 has a wrong check digit (should be …5).
    assert!(matches!(
        Gln::parse("4000001000002"),
        Err(GlnError::InvalidCheckDigit { .. })
    ));
    assert!(Gln::parse("4000001000005").is_ok());
}
