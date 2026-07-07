//! QR-code resolver URL construction.

use super::codec::percent_encode;

/// Build a GS1 Digital Link URL for a passport.
///
/// Uses the passport ID as the serial number (AI 21) and an optional
/// batch/lot (AI 10). AI values are percent-encoded.
pub fn build_qr_url(
    resolver_base: &str,
    gtin: &str,
    passport_id: &str,
    batch: Option<&str>,
) -> String {
    let mut uri = format!("{}/01/{}", resolver_base.trim_end_matches('/'), gtin);
    if let Some(b) = batch {
        uri.push_str(&format!("/10/{}", percent_encode(b)));
    }
    uri.push_str(&format!("/21/{}", percent_encode(passport_id)));
    uri
}
