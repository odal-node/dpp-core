//! GTIN validation for GS1 Digital Link primary keys.

use dpp_domain::Gtin;

use super::error::DigitalLinkError;

/// Validate a GTIN string (14 digits, correct GS1 mod-10 check digit).
///
/// Accepts GTIN-14 only. GTIN-8 / GTIN-12 / GTIN-13 should be normalised to
/// 14 digits before calling; [`super::link::DigitalLink::parse`] does this
/// automatically.
pub fn validate_gtin(gtin: &str) -> Result<(), DigitalLinkError> {
    Gtin::parse(gtin)
        .map(|_| ())
        .map_err(DigitalLinkError::from)
}
