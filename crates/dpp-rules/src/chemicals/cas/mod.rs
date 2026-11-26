//! CAS Registry Number format and check-digit validation.
//!
//! A valid CAS RN has the form `XXXXXXX-YY-Z` where:
//! - Segment 1: 2–7 ASCII digits (the registry number).
//! - Segment 2: exactly 2 ASCII digits.
//! - Segment 3: exactly 1 check digit.
//! - Check digit = `(Σ position_from_right × digit) mod 10` over the non-check digits.

mod format;
#[cfg(test)]
mod tests;

pub use format::*;
