//! Tests of the `codec` module.

use super::super::super::error::DigitalLinkError;
use super::super::*;

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
