//! CAS registry number format validation.

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
