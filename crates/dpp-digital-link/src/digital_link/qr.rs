//! QR-code resolver URL construction.

use super::codec::percent_encode;

/// Build a GS1 Digital Link URL for a passport carrier.
///
/// Encodes the GTIN (AI 01), an optional batch/lot (AI 10), and the item
/// `serial` (AI 21). The `serial` must be a GS1-conformant value: AI 21 is
/// capped at 20 characters, so a raw 36-character UUID cannot be used — derive
/// a conformant serial from a passport UUID with [`short_serial`]. AI values
/// are percent-encoded.
pub fn build_qr_url(resolver_base: &str, gtin: &str, serial: &str, batch: Option<&str>) -> String {
    let mut uri = format!("{}/01/{}", resolver_base.trim_end_matches('/'), gtin);
    if let Some(b) = batch {
        uri.push_str(&format!("/10/{}", percent_encode(b)));
    }
    uri.push_str(&format!("/21/{}", percent_encode(serial)));
    uri
}

/// Derive a GS1-conformant AI 21 serial from a passport UUID.
///
/// The GS1 General Specifications cap the AI 21 serial at 20 characters (a limit
/// the `DigitalLink` parser enforces), so a canonical
/// 36-character UUID cannot be carried directly. This encodes the first 10 bytes
/// of the UUID as lowercase hex: exactly 20 characters, drawn only from
/// `[0-9a-f]` (URL-safe and within the GS1 encodable character set). It is
/// non-sequential — so a public carrier leaks no production volume — and unique
/// per item at any realistic scale.
pub fn short_serial(uuid_bytes: &[u8; 16]) -> String {
    let mut serial = String::with_capacity(20);
    for &byte in &uuid_bytes[..10] {
        serial.push_str(&format!("{byte:02x}"));
    }
    serial
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::digital_link::{DigitalLink, DigitalLinkError};

    #[test]
    fn short_serial_is_twenty_hex_chars() {
        let uuid = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba,
            0xdc, 0xfe,
        ];
        let serial = short_serial(&uuid);
        assert_eq!(serial, "0123456789abcdef1032");
        assert_eq!(serial.len(), 20);
        assert!(serial.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn short_serial_is_deterministic() {
        let uuid = [0x7f; 16];
        assert_eq!(short_serial(&uuid), short_serial(&uuid));
    }

    #[test]
    fn derived_serial_round_trips_through_parse() {
        // The core invariant: build must never emit what parse rejects.
        let uuid = [0xa1; 16];
        let serial = short_serial(&uuid);
        let url = build_qr_url("https://id.odal-node.io", "09506000134352", &serial, None);
        let parsed = DigitalLink::parse(&url).expect("a derived serial must parse");
        assert_eq!(parsed.serial.as_deref(), Some(serial.as_str()));
        assert!(serial.chars().count() <= 20);
    }

    #[test]
    fn raw_uuid_serial_is_rejected_by_parse() {
        // Documents the defect the short serial fixes: a 36-char UUID in AI 21
        // exceeds the GS1 20-char cap, so the parser rejects the built URL.
        let url = build_qr_url(
            "https://id.odal-node.io",
            "09506000134352",
            "0190a9f0-1234-7abc-8def-0123456789ab", // 36 chars
            None,
        );
        assert!(matches!(
            DigitalLink::parse(&url),
            Err(DigitalLinkError::ValueTooLong { code, max_len: 20, actual: 36 }) if code == "21"
        ));
    }
}
