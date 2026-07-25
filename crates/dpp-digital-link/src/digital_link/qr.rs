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
/// the `DigitalLink` parser enforces), so a canonical 36-character UUID cannot be
/// carried directly. This encodes the **last 10 bytes** of the UUID as lowercase
/// hex: exactly 20 characters, drawn only from `[0-9a-f]` (URL-safe and within
/// the GS1 encodable character set), and deterministic for a given passport.
///
/// # Why the last ten bytes, not the first
///
/// Passport IDs are UUIDv7, whose leading six bytes are a big-endian millisecond
/// timestamp (RFC 9562 §5.7). Encoding the *first* ten bytes therefore produced a
/// serial that was monotonically increasing over time and whose first twelve hex
/// characters decoded directly to the passport's creation instant — so a QR code
/// on a physical battery published its creation time to the millisecond, and any
/// two codes revealed their production order and the rate between them. An
/// earlier version of this function did exactly that while its documentation
/// claimed the opposite.
///
/// Bytes 6..16 are the `rand_a` and `rand_b` fields: 74 random bits, with only
/// the 4-bit version and 2-bit variant fixed. No timestamp, no ordering.
pub fn short_serial(uuid_bytes: &[u8; 16]) -> String {
    let mut serial = String::with_capacity(20);
    for &byte in &uuid_bytes[6..] {
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
        // Bytes 6..16 — the UUIDv7 random tail, not the timestamp head.
        assert_eq!(serial, "cdef1032547698badcfe");
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

    /// Regression: the serial must not leak the passport's creation time.
    ///
    /// UUIDv7's leading six bytes are a big-endian millisecond timestamp. When
    /// the serial was cut from the *front* of the UUID, its first twelve hex
    /// characters were that timestamp verbatim.
    #[test]
    fn serial_does_not_embed_the_uuid_timestamp() {
        // A UUIDv7 whose timestamp bytes are a recognisable pattern.
        let mut uuid = [0u8; 16];
        uuid[..6].copy_from_slice(&[0x01, 0x9f, 0x99, 0xa0, 0xde, 0xad]);
        uuid[6] = 0x7a;
        uuid[7..].copy_from_slice(&[0xbc, 0x8d, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab]);

        let serial = short_serial(&uuid);
        assert!(
            !serial.contains("019f99a0dead"),
            "serial must not carry the UUIDv7 timestamp: {serial}"
        );
        assert_eq!(serial.len(), 20);
    }

    /// Regression: serials must not sort in creation order.
    ///
    /// Two batteries produced minutes apart would otherwise reveal their
    /// production order — and, across many codes, production volume — from the
    /// QR code alone.
    #[test]
    fn serials_do_not_order_by_creation_time() {
        // Two UUIDv7s with increasing timestamps but unrelated random tails.
        let mut earlier = [0u8; 16];
        earlier[..6].copy_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x01]);
        earlier[6..].copy_from_slice(&[0x7f, 0xff, 0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);

        let mut later = [0u8; 16];
        later[..6].copy_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x02]);
        later[6..].copy_from_slice(&[0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        assert!(
            short_serial(&later) < short_serial(&earlier),
            "a later UUID must be free to sort before an earlier one"
        );
    }

    #[test]
    fn serial_uses_the_random_tail_not_the_timestamp_head() {
        let mut a = [0u8; 16];
        a[..6].copy_from_slice(&[0xaa; 6]);
        a[6..].copy_from_slice(&[0x11; 10]);
        let mut b = [0u8; 16];
        b[..6].copy_from_slice(&[0xbb; 6]);
        b[6..].copy_from_slice(&[0x11; 10]);

        // Differing only in the timestamp head must not change the serial.
        assert_eq!(short_serial(&a), short_serial(&b));
        assert_eq!(short_serial(&a), "11".repeat(10));
    }
}
