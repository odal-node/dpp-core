//! GS1 key parsing: which strings are accepted, and which are refused and why.

use super::*;

// 09506000134352 — verified valid GTIN-14 used throughout the test suite.
const VALID: &str = "09506000134352";

#[test]
fn valid_gtin_parses() {
    assert!(Gtin::parse(VALID).is_ok());
}

#[test]
fn undefined_length_rejected() {
    // 11 digits is not a GTIN in any form, so it is refused rather than
    // padded — padding it would invent an identifier.
    assert!(matches!(
        Gtin::parse("09506000134"),
        Err(GtinError::InvalidFormat(_))
    ));
    assert!(matches!(
        Gtin::parse("095060001343526"),
        Err(GtinError::InvalidFormat(_))
    ));
}

#[test]
fn shorter_gs1_forms_normalise_to_fourteen() {
    // GS1 defines GTIN-8/12/13 as right-aligned in a 14-digit field, so all
    // three name a trade item the 14-digit form also names.
    for (input, canonical) in [
        ("3801234567898", "03801234567898"), // GTIN-13 (retail EAN-13)
        ("036000291452", "00036000291452"),  // GTIN-12 (UPC-A)
        ("12345670", "00000012345670"),      // GTIN-8
    ] {
        let g = Gtin::parse(input).unwrap_or_else(|e| panic!("{input} must parse: {e}"));
        assert_eq!(g.as_str(), canonical, "{input} must normalise");
    }
}

#[test]
fn a_short_form_and_its_padded_form_are_the_same_value() {
    // The point of normalising on parse: two spellings of one identifier
    // compare equal, so downstream lookups cannot miss on formatting alone.
    assert_eq!(
        Gtin::parse("3801234567898").unwrap(),
        Gtin::parse("03801234567898").unwrap()
    );
}

#[test]
fn a_short_form_with_a_bad_check_digit_is_still_refused() {
    // Normalisation is not leniency. Padding is lossless for the check
    // digit, so a wrong one stays wrong.
    assert!(matches!(
        Gtin::parse("3801234567890"),
        Err(GtinError::InvalidCheckDigit { .. })
    ));
}

#[test]
fn non_digits_rejected() {
    assert!(matches!(
        Gtin::parse("0950600013435X"),
        Err(GtinError::InvalidFormat(_))
    ));
}

#[test]
fn bad_check_digit_rejected() {
    // Last digit changed from 2 → 1: wrong check digit.
    assert!(matches!(
        Gtin::parse("09506000134351"),
        Err(GtinError::InvalidCheckDigit { .. })
    ));
}

#[test]
fn display_equals_inner_string() {
    let g = Gtin::parse(VALID).unwrap();
    assert_eq!(g.to_string(), VALID);
}

#[test]
fn partial_eq_str() {
    let g = Gtin::parse(VALID).unwrap();
    assert_eq!(g, *VALID); // PartialEq<str>: gtin == *str_ref
    assert_eq!(g.as_str(), VALID); // as_str() for direct &str comparison
}

#[test]
fn serde_round_trip() {
    let g = Gtin::parse(VALID).unwrap();
    let json = serde_json::to_string(&g).unwrap();
    assert_eq!(json, format!("\"{}\"", VALID));
    let back: Gtin = serde_json::from_str(&json).unwrap();
    assert_eq!(g, back);
}

#[test]
fn invalid_gtin_fails_deserialization() {
    // Check digit wrong.
    let result = serde_json::from_str::<Gtin>("\"09506000134351\"");
    assert!(result.is_err());
}

#[test]
fn prepend_zero_to_valid_ean13_gives_valid_gtin14() {
    // 1234567890128 is a valid EAN-13; prepending 0 gives a valid GTIN-14.
    assert!(Gtin::parse("01234567890128").is_ok());
}

// ── GLN ──────────────────────────────────────────────────────────────────

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

#[test]
fn gs1_check_digit_matches_known_keys() {
    // GTIN-14 and GLN both use the same mod-10 routine.
    assert_eq!(gs1_check_digit(&[0, 9, 5, 0, 6, 0, 0, 0, 1, 3, 4, 3, 5]), 2);
    assert_eq!(gs1_check_digit(&[4, 0, 1, 2, 3, 4, 5, 0, 0, 0, 0, 0]), 9);
}
