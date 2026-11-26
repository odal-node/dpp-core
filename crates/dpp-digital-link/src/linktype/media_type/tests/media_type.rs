//! Tests of the `media_type` module.

use super::super::*;

#[test]
fn parse_and_as_mime_round_trip_for_known_types() {
    let cases = [
        (DppMediaType::Json, "application/json"),
        (DppMediaType::JsonLd, "application/ld+json"),
        (DppMediaType::Html, "text/html"),
        (DppMediaType::Pdf, "application/pdf"),
        (DppMediaType::Cbor, "application/cbor"),
    ];
    for (variant, mime) in cases {
        assert_eq!(DppMediaType::parse(mime), variant);
        assert_eq!(variant.as_mime(), mime);
    }
}

#[test]
fn parse_strips_parameters() {
    assert_eq!(
        DppMediaType::parse("application/json; charset=utf-8"),
        DppMediaType::Json
    );
    // Leading/trailing whitespace around the base type is trimmed.
    assert_eq!(DppMediaType::parse("  text/html "), DppMediaType::Html);
}

#[test]
fn unknown_mime_becomes_custom() {
    let custom = DppMediaType::parse("application/x-protobuf");
    assert_eq!(
        custom,
        DppMediaType::Custom("application/x-protobuf".to_owned())
    );
    assert_eq!(custom.as_mime(), "application/x-protobuf");
}
