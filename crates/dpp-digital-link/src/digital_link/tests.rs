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

// ── G-5: official GS1 Digital Link worked examples ──────────────────────
//
// Source: GS1 Digital Link Standard — URI Syntax, Release 1.6.0 (Ratified,
// Mar 2025), §5 "Examples of GS1 Digital Link URIs" (an informative section
// explicitly meant as conformance reference). GTIN 9520123456788 is GS1's
// own worked example throughout; its mod-10 check digit was independently
// re-derived and confirmed valid before use here. Compressed-link forms
// (§6.1.2 of the same standard) are out of scope — this codec only
// implements the uncompressed path-segment form, so no vector here exercises
// compression. Data-attribute query AIs (net weight §5.6, amount payable
// §5.7) are likewise out of scope: this codec only models AI 01/22/10/21/235
// as structured fields, so those examples aren't representable here.
mod gs1_official_worked_examples {
    use super::*;

    /// §5.1 "GTIN" — canonical form on the id.gs1.org domain.
    #[test]
    fn section_5_1_gtin_canonical() {
        let uri = "https://id.gs1.org/01/09520123456788";
        let dl = DigitalLink::parse(uri).expect("GS1's own canonical example must parse");
        assert_eq!(dl.gtin.as_str(), "09520123456788");
        assert_eq!(dl.resolver_base, "https://id.gs1.org");
        assert_eq!(dl.build(), uri, "must round-trip to the canonical form");
    }

    /// §5.1 "GTIN" — non-canonical custom-domain form.
    #[test]
    fn section_5_1_gtin_custom_domain() {
        let uri = "https://brand.example.com/01/09520123456788";
        let dl = DigitalLink::parse(uri).expect("GS1's own custom-domain example must parse");
        assert_eq!(dl.gtin.as_str(), "09520123456788");
        assert_eq!(dl.build(), uri);
    }

    /// §5.1 "GTIN" — non-canonical form with a resolver path prefix before
    /// `/01/`. Exercises the path-prefix preservation this codec explicitly
    /// supports beyond the README's originally-advertised AI set.
    #[test]
    fn section_5_1_gtin_with_path_prefix() {
        let uri = "https://brand.example.com/some-extra/pathinfo/01/09520123456788";
        let dl = DigitalLink::parse(uri).expect("GS1's own path-prefix example must parse");
        assert_eq!(dl.gtin.as_str(), "09520123456788");
        assert_eq!(
            dl.resolver_base,
            "https://brand.example.com/some-extra/pathinfo"
        );
        assert_eq!(dl.build(), uri, "path prefix must round-trip exactly");
    }

    /// §5.2 "GTIN + CPV" — canonical form combined with AI 22.
    #[test]
    fn section_5_2_gtin_plus_cpv() {
        let uri = "https://id.gs1.org/01/09520123456788/22/2A";
        let dl = DigitalLink::parse(uri).expect("GS1's own CPV example must parse");
        assert_eq!(dl.gtin.as_str(), "09520123456788");
        assert_eq!(dl.variant.as_deref(), Some("2A"));
        assert_eq!(dl.build(), uri);
    }

    /// §5.3 "GTIN + Batch/Lot" — canonical form combined with AI 10.
    #[test]
    fn section_5_3_gtin_plus_batch_lot() {
        let uri = "https://id.gs1.org/01/09520123456788/10/ABC123";
        let dl = DigitalLink::parse(uri).expect("GS1's own batch/lot example must parse");
        assert_eq!(dl.gtin.as_str(), "09520123456788");
        assert_eq!(dl.batch.as_deref(), Some("ABC123"));
        assert_eq!(dl.build(), uri);
    }

    /// §5.4 "GTIN + Serial Number" (SGTIN) — canonical form combined with AI 21.
    #[test]
    fn section_5_4_gtin_plus_serial() {
        let uri = "https://id.gs1.org/01/09520123456788/21/12345";
        let dl = DigitalLink::parse(uri).expect("GS1's own serial-number example must parse");
        assert_eq!(dl.gtin.as_str(), "09520123456788");
        assert_eq!(dl.serial.as_deref(), Some("12345"));
        assert_eq!(dl.build(), uri);
    }

    /// §5.5 "GTIN + Batch/Lot + Serial Number + Expiry Date" — the canonical
    /// example combines AI 10 + AI 21 in the path with AI 17 (expiry) as a
    /// query parameter. AI 17 as a data-attribute query AI isn't modelled by
    /// this codec, so this vector confirms the query string is correctly
    /// stripped and ignored rather than corrupting the last path qualifier
    /// (the exact failure mode `mod.rs`'s doc comment calls out).
    #[test]
    fn section_5_5_gtin_plus_batch_plus_serial_query_string_ignored() {
        let uri = "https://id.gs1.org/01/09520123456788/10/ABC1/21/12345?17=180426";
        let dl = DigitalLink::parse(uri).expect("GS1's own batch+serial+expiry example must parse");
        assert_eq!(dl.gtin.as_str(), "09520123456788");
        assert_eq!(dl.batch.as_deref(), Some("ABC1"));
        assert_eq!(dl.serial.as_deref(), Some("12345"));
        // build() has no AI 17 support, so it reproduces the path without
        // the query string — still a valid, equivalent GS1 Digital Link URI.
        assert_eq!(
            dl.build(),
            "https://id.gs1.org/01/09520123456788/10/ABC1/21/12345"
        );
    }
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

#[test]
fn trailing_unpaired_ai_rejected() {
    // AI 21 with no following value must error, not silently drop the serial.
    let uri = "https://id.odal-node.io/01/09506000134352/21";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::TrailingUnpairedSegment(_))
    ));
}

#[test]
fn duplicate_primary_key_rejected() {
    // A second '01' segment must not overwrite the GTIN from the first.
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN1/01/00000000000001";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::DuplicatePrimaryKey)
    ));
}

#[test]
fn oversized_ai_value_rejected() {
    // AI 21 (serial) has a GS1 max length of 20; a 21-char value must error.
    let long_serial = "X".repeat(21);
    let uri = format!("https://id.odal-node.io/01/09506000134352/21/{long_serial}");
    assert!(matches!(
        DigitalLink::parse(&uri),
        Err(DigitalLinkError::ValueTooLong { code, max_len, actual })
            if code == "21" && max_len == 20 && actual == 21
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
