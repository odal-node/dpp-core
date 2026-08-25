//! The shared GS1 modulo-10 check digit, and the fixed-length key check both
//! [`Gtin`](super::gtin::Gtin) and [`Gln`](super::gln::Gln) are built on.

/// Compute the GS1 modulo-10 check digit for the *data* portion of a GS1 key.
///
/// Shared by GTIN-14, GLN-13 and other fixed-length GS1 numeric keys. The
/// rightmost data digit carries weight 3, then alternating 1,3,… leftward
/// (the canonical GS1 rule). `data_digits` holds values 0–9 and excludes the
/// trailing check digit.
#[must_use]
pub fn gs1_check_digit(data_digits: &[u8]) -> u8 {
    let sum: u32 = data_digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| u32::from(d) * if i % 2 == 0 { 3 } else { 1 })
        .sum();
    ((10 - (sum % 10)) % 10) as u8
}

/// Outcome of the shared fixed-length GS1 key check, before it's wrapped in a
/// key-specific error type (`GtinError`/`GlnError`) by each caller.
pub(super) enum Gs1KeyCheck {
    InvalidFormat,
    InvalidCheckDigit { expected: u8, actual: u8 },
}

/// Validate a fixed-length numeric GS1 key: exactly `len` ASCII digits with a
/// correct GS1 modulo-10 check digit. Shared by [`Gtin::parse`] (`len = 14`)
/// and [`Gln::parse`] (`len = 13`) — the two differ only in length and in
/// which error type the caller wraps the result into.
pub(super) fn check_gs1_key(s: &str, len: usize) -> Result<(), Gs1KeyCheck> {
    if s.len() != len || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Gs1KeyCheck::InvalidFormat);
    }
    // Stack buffer, not a Vec: every GS1 key this is called with (GTIN-14,
    // GLN-13) fits comfortably within 14 digits.
    debug_assert!(
        len <= 14,
        "check_gs1_key only supports keys up to 14 digits"
    );
    let mut digits = [0u8; 14];
    for (i, b) in s.bytes().enumerate() {
        digits[i] = b - b'0';
    }
    let expected = gs1_check_digit(&digits[..len - 1]);
    if digits[len - 1] != expected {
        return Err(Gs1KeyCheck::InvalidCheckDigit {
            expected,
            actual: digits[len - 1],
        });
    }
    Ok(())
}
