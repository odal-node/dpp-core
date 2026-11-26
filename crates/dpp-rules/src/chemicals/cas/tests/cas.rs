//! Tests of the `cas` module.

use super::super::*;

#[test]
fn well_known_cas_numbers_valid() {
    for cas in &[
        "80-05-7",   // Bisphenol A
        "117-81-7",  // DEHP
        "84-74-2",   // DBP
        "79-06-1",   // Acrylamide
        "7440-43-9", // Cadmium
        "1333-82-0", // Chromium trioxide
        "872-50-4",  // NMP
        "7440-66-6", // Zinc (valid format, not an SVHC)
    ] {
        assert!(validate_cas_format(cas).is_ok(), "expected valid: {cas}");
    }
}

#[test]
fn wrong_segment_count_rejected() {
    assert!(validate_cas_format("8057").is_err());
    assert!(validate_cas_format("80-057").is_err());
    assert!(validate_cas_format("80-05-7-extra").is_err());
    assert!(validate_cas_format("").is_err());
}

#[test]
fn wrong_segment_lengths_rejected() {
    assert!(validate_cas_format("8-05-7").is_err()); // reg too short
    assert!(validate_cas_format("123456789-05-7").is_err()); // reg too long (>7 digits)
    assert!(validate_cas_format("80-5-7").is_err()); // mid only 1 digit
    assert!(validate_cas_format("80-057-7").is_err()); // mid 3 digits
    assert!(validate_cas_format("80-05-77").is_err()); // check 2 digits
}

#[test]
fn invalid_check_digit_rejected() {
    // BPA is 80-05-7; all other digits are wrong.
    assert!(validate_cas_format("80-05-8").is_err());
    assert!(validate_cas_format("80-05-0").is_err());
    assert!(validate_cas_format("80-05-6").is_err());
}

#[test]
fn non_digit_characters_rejected() {
    assert!(validate_cas_format("8X-05-7").is_err());
    assert!(validate_cas_format("80-0A-7").is_err());
    assert!(validate_cas_format("80-05-Z").is_err());
}
