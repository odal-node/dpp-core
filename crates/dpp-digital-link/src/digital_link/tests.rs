//! Parser/builder, GTIN-validation, normalisation, and encoding tests.

use super::*;

#[test]
fn round_trip_gtin_serial() {
    let uri = "https://id.odal-node.io/01/09506000134352/21/ABC123";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin.as_str(), "09506000134352");
    assert_eq!(dl.serial.as_deref(), Some("ABC123"));
    assert_eq!(dl.build(), uri);
}

#[test]
fn non_https_scheme_is_rejected() {
    assert!(matches!(
        DigitalLink::parse("http://id.odal-node.io/01/09506000134352"),
        Err(DigitalLinkError::InvalidScheme(_))
    ));
}

#[test]
fn round_trips_all_qualifiers_in_canonical_order() {
    // 22 (variant) → 10 (batch) → 21 (serial) → 235 (third-party serial),
    // the canonical ascending qualifier order.
    let uri = "https://id.odal-node.io/01/09506000134352/22/VAR-1/10/LOT-9/21/SN-7/235/TPX-3";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.variant.as_deref(), Some("VAR-1"));
    assert_eq!(dl.batch.as_deref(), Some("LOT-9"));
    assert_eq!(dl.serial.as_deref(), Some("SN-7"));
    assert_eq!(dl.tpcsn.as_deref(), Some("TPX-3"));
    // build() emits the variant (22) and tpcsn (235) branches too.
    assert_eq!(dl.build(), uri);
}

#[test]
fn parse_with_batch() {
    let uri = "https://id.odal-node.io/01/09506000134352/10/BATCH01/21/SN001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.batch.as_deref(), Some("BATCH01"));
    assert_eq!(dl.serial.as_deref(), Some("SN001"));
}

#[test]
fn invalid_gtin_rejected() {
    let uri = "https://id.odal-node.io/01/123/21/SN1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::InvalidGtin(_))
    ));
}

#[test]
fn missing_gtin_rejected() {
    let uri = "https://id.odal-node.io/21/SN1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::MissingGtin)
    ));
}

// ── validate_gtin tests ────────────────────────────────────────

#[test]
fn validate_gtin_valid() {
    assert!(validate_gtin("09506000134352").is_ok());
}

#[test]
fn validate_gtin_wrong_length() {
    assert!(matches!(
        validate_gtin("123456"),
        Err(DigitalLinkError::InvalidGtin(_))
    ));
}

#[test]
fn validate_gtin_non_digits() {
    assert!(matches!(
        validate_gtin("0950600013435X"),
        Err(DigitalLinkError::InvalidGtin(_))
    ));
}

#[test]
fn validate_gtin_bad_check_digit() {
    assert!(matches!(
        validate_gtin("09506000134351"),
        Err(DigitalLinkError::InvalidGtinCheckDigit { .. })
    ));
}

#[test]
fn build_qr_url_with_batch() {
    let url = build_qr_url(
        "https://id.odal-node.io",
        "09506000134352",
        "passport-123",
        Some("BATCH-01"),
    );
    assert_eq!(
        url,
        "https://id.odal-node.io/01/09506000134352/10/BATCH-01/21/passport-123"
    );
}

#[test]
fn build_qr_url_without_batch() {
    let url = build_qr_url(
        "https://id.odal-node.io",
        "09506000134352",
        "passport-456",
        None,
    );
    assert_eq!(
        url,
        "https://id.odal-node.io/01/09506000134352/21/passport-456"
    );
}

#[test]
fn bad_check_digit_rejected_via_parse() {
    let uri = "https://id.odal-node.io/01/09506000134351/21/SN1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::InvalidGtinCheckDigit { .. })
    ));
}

// ── 3.4a: query string ────────────────────────────────────────

#[test]
fn query_string_does_not_corrupt_serial() {
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN001?linkType=gs1:pip";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.serial.as_deref(), Some("SN001"));
}

#[test]
fn query_only_uri_returns_gtin_cleanly() {
    let uri = "https://id.odal-node.io/01/09506000134352?linkType=gs1:pip";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin.as_str(), "09506000134352");
    assert_eq!(dl.serial, None);
}

// ── 3.4a: GTIN normalisation ──────────────────────────────────

#[test]
fn gtin_13_normalised_to_14() {
    // EAN-13 5901234123457 → GTIN-14 05901234123457 (check digit preserved)
    let uri = "https://id.odal-node.io/01/5901234123457/21/SN1";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin.as_str(), "05901234123457");
}

#[test]
fn gtin_12_normalised_to_14() {
    // UPC-A 012345678905 → GTIN-14 00012345678905
    let uri = "https://id.odal-node.io/01/012345678905/21/SN1";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin.as_str(), "00012345678905");
}

#[test]
fn gtin_of_invalid_length_rejected() {
    let uri = "https://id.odal-node.io/01/123456789/21/SN1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::InvalidGtin(_))
    ));
}

// ── 3.4a: unknown / misordered AI rejection ───────────────────

#[test]
fn unknown_ai_rejected() {
    let uri = "https://id.odal-node.io/01/09506000134352/99/unknown/21/SN1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::UnknownApplicationIdentifier(_))
    ));
}

#[test]
fn misordered_qualifiers_rejected() {
    // AI 21 (order 3) before AI 10 (order 2) violates canonical order.
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN1/10/BATCH1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::QualifiersOutOfOrder { .. })
    ));
}

// ── 3.4a: percent-encode / decode ────────────────────────────

#[test]
fn percent_encoded_serial_decoded_on_parse() {
    // %2F is '/'
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN%2F001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.serial.as_deref(), Some("SN/001"));
}

#[test]
fn slash_in_serial_encoded_on_build() {
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN1";
    let mut dl = DigitalLink::parse(uri).unwrap();
    dl.serial = Some("SN/001".to_owned());
    assert!(dl.build().contains("/21/SN%2F001"));
}

#[test]
fn question_mark_in_serial_encoded_on_build() {
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN1";
    let mut dl = DigitalLink::parse(uri).unwrap();
    dl.serial = Some("SN?001".to_owned());
    assert!(dl.build().contains("/21/SN%3F001"));
}

// ── 3.4b: path-prefix resolver round-trip ────────────────────

#[test]
fn path_prefixed_resolver_round_trip() {
    let uri = "https://example.com/resolve/01/09506000134352/21/SN001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.resolver_base, "https://example.com/resolve");
    assert_eq!(dl.build(), uri);
}

#[test]
fn multi_segment_path_prefix_preserved() {
    let uri = "https://example.com/api/v1/dpp/01/09506000134352/21/SN001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.resolver_base, "https://example.com/api/v1/dpp");
    assert_eq!(dl.build(), uri);
}

#[test]
fn port_bearing_resolver_round_trip() {
    let uri = "https://localhost:8080/01/09506000134352/21/SN001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.resolver_base, "https://localhost:8080");
    assert_eq!(dl.build(), uri);
}

#[test]
fn path_prefixed_resolver_with_batch_round_trip() {
    let uri = "https://example.com/resolve/01/09506000134352/10/LOT1/21/SN001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.resolver_base, "https://example.com/resolve");
    assert_eq!(dl.build(), uri);
}
