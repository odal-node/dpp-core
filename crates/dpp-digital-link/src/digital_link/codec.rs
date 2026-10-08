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

/// Decode the percent escapes in a URI path segment value.
///
/// A `%` must be followed by two hexadecimal digits, and the decoded bytes must
/// be UTF-8. Anything else is an error rather than being passed through, since a
/// value read as something other than what the URI spelled is a different
/// identifier from the one printed on the label.
pub(super) fn percent_decode(s: &str) -> Result<String, DigitalLinkError> {
    let malformed = || DigitalLinkError::MalformedPercentEscape(s.to_owned());
    let bytes = s.as_bytes();
    let mut result: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            result.push(bytes[i]);
            i += 1;
            continue;
        }
        let hex = bytes.get(i + 1..i + 3).ok_or_else(malformed)?;
        let hex = std::str::from_utf8(hex).map_err(|_| malformed())?;
        // `from_str_radix` accepts a leading `+`, which an escape does not.
        if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(malformed());
        }
        result.push(u8::from_str_radix(hex, 16).map_err(|_| malformed())?);
        i += 3;
    }
    String::from_utf8(result).map_err(|_| malformed())
}

/// Percent-encode a string for use as a GS1 DL URI path segment value.
///
/// Everything outside RFC 3986's unreserved set is encoded as `%XX`. That is
/// narrower than a path segment allows, on purpose: GS1's grammar spells every
/// symbol of the 82-character set that is not `-`, `.` or `_` as its escape, so
/// a value leaves here in the one form the grammar names. `!`, `&`, `'`, `(`,
/// `)`, `*`, `+`, `,`, `;`, `=` and `:` are legal in a path under RFC 3986, and
/// the parser still accepts them raw, but they are not what the grammar says.
pub(super) fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for &byte in s.as_bytes() {
        match byte {
            // Unreserved (RFC 3986 §2.3)
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push_str(&format!("{byte:02X}"));
            }
        }
    }
    result
}

/// Whether `authority` is a host, and an optional port, a Web URI can have.
///
/// The host is a registered name, an IPv4 address, or a bracketed IPv6 address
/// or IPvFuture literal, and the port is digits: RFC 3986 section 3.2.2, which
/// GS1's grammar restates. A user name and an `@` are not part of it.
pub(super) fn is_valid_authority(authority: &str) -> bool {
    let (host, port) = if let Some(literal) = authority.strip_prefix('[') {
        let Some((inside, rest)) = literal.split_once(']') else {
            return false;
        };
        if !is_ip_literal(inside) {
            return false;
        }
        (
            None,
            rest.strip_prefix(':').or(rest.is_empty().then_some("")),
        )
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (Some(host), Some(port)),
            None => (Some(authority), Some("")),
        }
    };
    let port_ok = port.is_some_and(|p| p.bytes().all(|b| b.is_ascii_digit()));
    // A grammar `reg-name` may be empty, but a base with no host names no
    // resolver, and one cannot be written back out: `build` trims the slashes
    // that would then follow the scheme.
    let host_ok = host.is_none_or(|h| !h.is_empty() && is_reg_name(h));
    port_ok && host_ok
}

/// Whether `segment` is a path segment: RFC 3986's `*pchar`, which GS1's grammar
/// restates, where `pchar = unreserved / pct-encoded / sub-delims / ":" / "@"`.
///
/// `also` admits characters the grammar allows beyond that in one position: the
/// value of an AI may hold the double quote, which `XSYMBOL` names as itself.
/// Everything else outside `pchar` (a space, `<`, `>`, `#`, `[`, a character
/// beyond ASCII) is not a character a path segment can hold raw.
pub(super) fn is_segment(segment: &str, also: &[u8]) -> bool {
    let bytes = segment.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit) =>
            {
                i += 3;
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => i += 1,
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' => i += 1,
            b':' | b'@' => i += 1,
            b if also.contains(&b) => i += 1,
            _ => return false,
        }
    }
    true
}

/// `reg-name = *( unreserved / pct-encoded / sub-delims )`.
fn is_reg_name(host: &str) -> bool {
    let bytes = host.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => i += 1,
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' => i += 1,
            b'%' if bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit) =>
            {
                i += 3;
            }
            _ => return false,
        }
    }
    true
}

/// The inside of `[` and `]`: an IPv6 address or an IPvFuture literal.
fn is_ip_literal(inside: &str) -> bool {
    if let Some(rest) = inside.strip_prefix(['v', 'V']) {
        let Some((version, tail)) = rest.split_once('.') else {
            return false;
        };
        return !version.is_empty()
            && version.bytes().all(|b| b.is_ascii_hexdigit())
            && !tail.is_empty()
            && tail
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:".contains(&b));
    }
    is_ipv6(inside)
}

/// RFC 3986's `IPv6address`: eight 16-bit groups, one run of which may be
/// written `::`, and the last two of which may be an IPv4 address.
fn is_ipv6(address: &str) -> bool {
    let groups_of = |part: &str| -> Option<usize> {
        if part.is_empty() {
            return Some(0);
        }
        let pieces: Vec<&str> = part.split(':').collect();
        let mut count = 0;
        for (index, piece) in pieces.iter().enumerate() {
            if index + 1 == pieces.len() && piece.contains('.') {
                if !is_ipv4(piece) {
                    return None;
                }
                count += 2;
            } else if (1..=4).contains(&piece.len()) && piece.bytes().all(|b| b.is_ascii_hexdigit())
            {
                count += 1;
            } else {
                return None;
            }
        }
        Some(count)
    };
    match address.split_once("::") {
        None => groups_of(address) == Some(8),
        Some((head, tail)) => {
            if tail.contains("::") {
                return false;
            }
            match (groups_of(head), groups_of(tail)) {
                (Some(h), Some(t)) => h + t <= 7,
                _ => false,
            }
        }
    }
}

/// Four decimal octets, each 0 to 255 with no leading zero.
fn is_ipv4(address: &str) -> bool {
    let octets: Vec<&str> = address.split('.').collect();
    octets.len() == 4
        && octets.iter().all(|o| {
            !o.is_empty()
                && o.len() <= 3
                && o.bytes().all(|b| b.is_ascii_digit())
                && (o.len() == 1 || !o.starts_with('0'))
                && o.parse::<u16>().is_ok_and(|n| n <= 255)
        })
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
        // Everything outside the unreserved set is escaped, `+` and `:` included:
        // they are legal in a path, but GS1's grammar spells them as escapes.
        let encoded = percent_encode("a/b c+d:e");
        assert_eq!(encoded, "a%2Fb%20c%2Bd%3Ae");
        assert_eq!(percent_decode(&encoded).unwrap(), "a/b c+d:e");
    }

    #[test]
    fn percent_decode_passes_through_plain_text() {
        assert_eq!(percent_decode("PLAIN-text_123").unwrap(), "PLAIN-text_123");
    }

    #[test]
    fn percent_decode_reads_either_case_of_hex() {
        assert_eq!(percent_decode("a%2fb%2Fc").unwrap(), "a/b/c");
    }

    #[test]
    fn percent_decode_refuses_what_is_not_an_escape() {
        for bad in ["A%ZZB", "A%2", "A%", "A%+1B", "A%FFB", "A%C3"] {
            assert!(
                matches!(
                    percent_decode(bad),
                    Err(DigitalLinkError::MalformedPercentEscape(_))
                ),
                "{bad} must not decode"
            );
        }
    }

    #[test]
    fn segments_are_pchar() {
        for good in ["", "resolve", "a%20b", "r:e@s!$&'()*+,;=", "A-B.C_D~E"] {
            assert!(is_segment(good, b""), "{good:?} is a segment");
        }
        for bad in [
            "a b", "a\"b", "a<b", "a>b", "a#b", "a?b", "a{b", "a\\b", "a%2", "a%zz", "\u{e9}",
        ] {
            assert!(!is_segment(bad, b""), "{bad:?} is not a segment");
        }
        assert!(
            is_segment("A\"B", b"\""),
            "a value may hold the double quote"
        );
    }

    #[test]
    fn authorities_follow_the_uri_grammar() {
        for good in [
            "example.com",
            "example.com:8443",
            "example.com:",
            "id.gs1.org",
            "127.0.0.1:80",
            "[::1]",
            "[2001:db8::8a2e:370:7334]:443",
            "[::ffff:192.0.2.1]",
            "[v7.a:b]",
            "my_host~name.example",
            "a%20b.example",
        ] {
            assert!(is_valid_authority(good), "{good} is a valid authority");
        }
        for bad in [
            "user@example.com",
            "example.com:80a",
            "example.com#frag",
            "",
            ":80",
            "exa mple.com",
            "[::1",
            "[1:2:3:4:5:6:7:8:9]",
            "[1::2::3]",
            "[12345::1]",
            "[::1]x",
            "[::256.0.0.1]",
            "bücher.example",
        ] {
            assert!(!is_valid_authority(bad), "{bad} is not a valid authority");
        }
    }
}
