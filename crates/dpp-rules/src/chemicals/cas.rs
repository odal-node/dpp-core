//! CAS Registry Number format and check-digit validation.
//!
//! A valid CAS RN has the form `XXXXXXX-YY-Z` where:
//! - Segment 1: 2–7 ASCII digits (the registry number).
//! - Segment 2: exactly 2 ASCII digits.
//! - Segment 3: exactly 1 check digit.
//! - Check digit = `(Σ position_from_right × digit) mod 10` over the non-check digits.

use alloc::{format, string::String};

/// Validate CAS Registry Number format and check digit.
///
/// Returns `Ok(())` for a well-formed CAS RN; `Err` with a descriptive message otherwise.
pub fn validate_cas_format(cas: &str) -> Result<(), String> {
    let mut it = cas.split('-');
    let reg = it.next().unwrap_or("");
    let mid = it.next().unwrap_or("");
    let chk = it.next().unwrap_or("");

    // Must be exactly 3 hyphen-separated segments.
    if it.next().is_some() || chk.is_empty() {
        return Err(format!(
            "'{cas}': CAS number must have exactly 3 segments (e.g. 80-05-7)"
        ));
    }

    if reg.len() < 2 || reg.len() > 7 || !reg.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "'{cas}': first segment must be 2–7 digits, got '{reg}'"
        ));
    }
    if mid.len() != 2 || !mid.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "'{cas}': second segment must be exactly 2 digits, got '{mid}'"
        ));
    }
    if chk.len() != 1 || !chk.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "'{cas}': third segment must be exactly 1 digit, got '{chk}'"
        ));
    }

    // Weighted sum over non-check digits, positions numbered from the right starting at 1.
    let expected = chk.as_bytes()[0] - b'0';
    let n = reg.len() + mid.len();
    let sum: u32 = reg
        .bytes()
        .chain(mid.bytes())
        .enumerate()
        .map(|(i, b)| (n - i) as u32 * u32::from(b - b'0'))
        .sum();
    let computed = (sum % 10) as u8;

    if computed != expected {
        return Err(format!(
            "'{cas}': check digit is {expected} but should be {computed}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
