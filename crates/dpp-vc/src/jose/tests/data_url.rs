//! The data of a `data:` URL, and the WHATWG Infra Standard's "forgiving-base64 decode" it is
//! read with.

use base64::Engine as _;

use super::super::data_url::{decode_data, decode_text, forgiving_base64_decode, split_data_url};
use crate::test_support::{cose_input, token};

#[test]
fn the_standards_own_example_decodes_both_final_characters_to_the_same_byte() {
    // "The discarded bits mean that, for instance, "YQ" and "YR" both return `a`."
    assert_eq!(forgiving_base64_decode("YQ"), Some(b"a".to_vec()));
    assert_eq!(forgiving_base64_decode("YR"), Some(b"a".to_vec()));
}

#[test]
fn a_quantum_may_end_with_one_or_two_padding_characters_or_none() {
    for (text, bytes) in [
        ("", &b""[..]),
        ("YQ==", b"a"),
        ("YQ", b"a"),
        ("YWI=", b"ab"),
        ("YWI", b"ab"),
        ("YWJj", b"abc"),
        ("YWJjZA==", b"abcd"),
        ("YWJjZA", b"abcd"),
    ] {
        assert_eq!(
            forgiving_base64_decode(text).as_deref(),
            Some(bytes),
            "{text}"
        );
    }
}

#[test]
fn it_agrees_with_the_padded_and_unpadded_encodings_of_every_short_input() {
    let standard = base64::engine::general_purpose::STANDARD;
    let unpadded = base64::engine::general_purpose::STANDARD_NO_PAD;
    let bytes: Vec<u8> = (0..=255).collect();
    for length in 0..=64 {
        let input = &bytes[..length];
        assert_eq!(
            forgiving_base64_decode(&standard.encode(input)).as_deref(),
            Some(input)
        );
        assert_eq!(
            forgiving_base64_decode(&unpadded.encode(input)).as_deref(),
            Some(input)
        );
    }
}

/// ASCII whitespace is U+0009, U+000A, U+000C, U+000D and U+0020, and is removed first, so a
/// base64 text may be wrapped in lines. U+000B and U+00A0 are not ASCII whitespace.
#[test]
fn ascii_whitespace_is_removed_and_nothing_else_is() {
    assert_eq!(
        forgiving_base64_decode(" Y\tW\nI\r=\u{000C}"),
        Some(b"ab".to_vec())
    );
    assert_eq!(forgiving_base64_decode("YW\nJj\n"), Some(b"abc".to_vec()));
    assert_eq!(forgiving_base64_decode("YW\u{000B}Jj"), None);
    assert_eq!(forgiving_base64_decode("YW\u{00A0}Jj"), None);
}

#[test]
fn a_length_of_one_modulo_four_fails() {
    for text in ["Y", "YWJjZ", "YWJjZ===", "YWJjZGVmZ"] {
        assert_eq!(forgiving_base64_decode(text), None, "{text}");
    }
}

#[test]
fn padding_is_accepted_only_at_the_end_of_a_whole_number_of_quanta() {
    for text in [
        "YQ=",      // three characters: no padding is removed, and `=` is not in the alphabet
        "YQ===",    // five characters
        "YQ==YQ==", // padding inside
        "====", "=", "=YQ=", "Y=Q=",
    ] {
        assert_eq!(forgiving_base64_decode(text), None, "{text}");
    }
}

#[test]
fn a_character_outside_the_standard_alphabet_fails() {
    // The URL alphabet of RFC 4648 §5, a space inside a quantum that is not whitespace-free,
    // a non-ASCII letter, a control character and a percent sign.
    for text in [
        "YW-j",
        "YW_j",
        "YW.j",
        "YW,j",
        "YWé",
        "YW\u{0001}j",
        "YW%4A",
    ] {
        assert_eq!(forgiving_base64_decode(text), None, "{text:?}");
    }
}

/// Row 7 of the suite envelops, as a `vc+cose`, row 29's message, written as 878 characters of
/// standard base64 with no padding and a line feed after them. RFC 2045 would need `==` at the
/// end. The data decodes to row 29's bytes.
#[test]
fn the_suites_row_7_envelope_data_is_read() {
    let presentation = token("presentation-jose-multiple.txt");
    let payload = presentation.split('.').nth(1).expect("a payload");
    let payload: serde_json::Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload)
            .expect("base64url"),
    )
    .expect("JSON");
    let id = payload["verifiableCredential"][2]["id"]
        .as_str()
        .expect("an id");

    let url = split_data_url(id).expect("a data: URL");
    assert_eq!(url.media_type, "application/vc+cose");
    assert!(url.base64);
    // The two properties that make it not RFC 2045 base64.
    assert!(url.data.ends_with('\n'));
    assert_eq!(url.data.trim_end().len() % 4, 2, "unpadded");
    assert_eq!(url.data.trim_end().len(), 878);

    assert_eq!(
        decode_data(url.data, true).expect("read"),
        cose_input("credential-cose-minimal.txt")
    );
    // The strict engine refuses it, which is why the read is forgiving.
    assert!(
        base64::engine::general_purpose::STANDARD
            .decode(url.data.trim_end())
            .is_err()
    );
}

#[test]
fn data_that_is_not_base64_is_refused_with_a_reason_and_none_of_the_data() {
    assert_eq!(decode_data("a-b_c", true), Err("data that is not base64"));
    assert_eq!(
        decode_data("%zz", false),
        Err("a percent escape that is not two hex digits")
    );
    assert_eq!(
        decode_text("%ff%fe", false),
        Err("data that is not UTF-8 text")
    );
}

#[test]
fn percent_escapes_are_decoded_before_base64_is() {
    // `%49` is `I`: "YWI=" is "ab".
    assert_eq!(decode_data("YW%49=", true), Ok(b"ab".to_vec()));
    // Without `;base64` the escapes are the bytes.
    assert_eq!(decode_data("YW%49=", false), Ok(b"YWI=".to_vec()));
    assert_eq!(decode_data("a%20b", false), Ok(b"a b".to_vec()));
}

/// RFC 3986 §2.1: a percent escape is "%" HEXDIG HEXDIG, in either case. `u8::from_str_radix`
/// also reads a leading `+`, so `%+A` would otherwise become a line feed, which the base64
/// decoder then removes as whitespace.
#[test]
fn a_percent_escape_is_exactly_two_hex_digits() {
    let refused = Err("a percent escape that is not two hex digits");
    for data in ["YWJj%+A", "%+a", "%-1", "% A", "%A", "%"] {
        assert_eq!(decode_data(data, true), refused, "{data}");
        assert_eq!(decode_data(data, false), refused, "{data}");
    }
    assert_eq!(decode_data("YWJj%0A", true), Ok(b"abc".to_vec()));
    assert_eq!(decode_data("%0a%0A", false), Ok(b"\n\n".to_vec()));
}

#[test]
fn a_data_url_splits_into_media_type_parameters_and_data() {
    let url = split_data_url("DATA:Application/VC+COSE;charset=x;base64,AAA=").expect("a URL");
    assert_eq!(url.media_type, "application/vc+cose");
    assert!(url.base64);
    assert_eq!(url.data, "AAA=");
    let url = split_data_url("data:,x").expect("a URL");
    assert_eq!(url.media_type, "text/plain");
    assert!(!url.base64);
    assert!(split_data_url("https://example.org").is_none());
    assert!(split_data_url("data:text/plain").is_none());
}
