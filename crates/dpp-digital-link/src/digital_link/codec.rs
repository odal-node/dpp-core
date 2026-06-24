//! URI value codec helpers: GTIN normalisation and percent-encoding.

use super::error::DigitalLinkError;

/// Left-pad a GTIN string to 14 digits (GTIN-8, -12, -13 → GTIN-14).
pub(super) fn normalize_gtin_to_14(s: &str) -> Result<String, DigitalLinkError> {
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DigitalLinkError::InvalidGtin(s.to_owned()));
    }
    match s.len() {
        8 | 12 | 13 => Ok(format!("{:0>14}", s)),
        14 => Ok(s.to_owned()),
        _ => Err(DigitalLinkError::InvalidGtin(s.to_owned())),
    }
}

/// Decode percent-encoded bytes in a URI path segment value.
pub(super) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            result.push(byte);
            i += 3;
            continue;
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

/// Percent-encode a string for use as a GS1 DL URI path segment value.
///
/// Characters in `unreserved / sub-delims / ":" / "@"` per RFC 3986 §3.3
/// are passed through unchanged; all others are encoded as `%XX`.
pub(super) fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for &byte in s.as_bytes() {
        match byte {
            // Unreserved (RFC 3986 §2.3)
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~'
            // Sub-delimiters safe in path segments
            | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
            // Path-only chars
            | b':' | b'@'
            => result.push(byte as char),
            _ => {
                result.push('%');
                result.push_str(&format!("{byte:02X}"));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_pads_short_gtins_to_14() {
        assert_eq!(normalize_gtin_to_14("12345678").unwrap(), "00000012345678");
        assert_eq!(
            normalize_gtin_to_14("123456789012").unwrap(),
            "00123456789012"
        );
        assert_eq!(
            normalize_gtin_to_14("09506000134352").unwrap(),
            "09506000134352"
        );
    }

    #[test]
    fn normalize_rejects_non_digits_and_bad_length() {
        assert!(matches!(
            normalize_gtin_to_14("12A45678"),
            Err(DigitalLinkError::InvalidGtin(_))
        ));
        assert!(matches!(
            normalize_gtin_to_14("123"),
            Err(DigitalLinkError::InvalidGtin(_))
        ));
    }

    #[test]
    fn percent_encode_decode_round_trip() {
        // space → %20, '/' → %2F; '+' and ':' are passed through (sub-delim / path).
        let encoded = percent_encode("a/b c+d:e");
        assert!(encoded.contains("%2F"));
        assert!(encoded.contains("%20"));
        assert!(encoded.contains('+'));
        assert!(encoded.contains(':'));
        assert_eq!(percent_decode(&encoded), "a/b c+d:e");
    }

    #[test]
    fn percent_decode_passes_through_plain_text() {
        assert_eq!(percent_decode("PLAIN-text_123"), "PLAIN-text_123");
    }
}
