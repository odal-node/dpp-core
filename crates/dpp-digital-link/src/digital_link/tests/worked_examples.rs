//! GS1's own worked examples, and GTIN validation.

use super::super::*;

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
        assert_eq!(dl.gtin().unwrap().as_str(), "09520123456788");
        assert_eq!(dl.resolver_base, "https://id.gs1.org");
        assert_eq!(dl.build(), uri, "must round-trip to the canonical form");
    }

    /// §5.1 "GTIN" — non-canonical custom-domain form.
    #[test]
    fn section_5_1_gtin_custom_domain() {
        let uri = "https://brand.example.com/01/09520123456788";
        let dl = DigitalLink::parse(uri).expect("GS1's own custom-domain example must parse");
        assert_eq!(dl.gtin().unwrap().as_str(), "09520123456788");
        assert_eq!(dl.build(), uri);
    }

    /// §5.1 "GTIN" — non-canonical form with a resolver path prefix before
    /// `/01/`. Exercises the path-prefix preservation this codec explicitly
    /// supports beyond the README's originally-advertised AI set.
    #[test]
    fn section_5_1_gtin_with_path_prefix() {
        let uri = "https://brand.example.com/some-extra/pathinfo/01/09520123456788";
        let dl = DigitalLink::parse(uri).expect("GS1's own path-prefix example must parse");
        assert_eq!(dl.gtin().unwrap().as_str(), "09520123456788");
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
        assert_eq!(dl.gtin().unwrap().as_str(), "09520123456788");
        assert_eq!(dl.variant(), Some("2A"));
        assert_eq!(dl.build(), uri);
    }

    /// §5.3 "GTIN + Batch/Lot" — canonical form combined with AI 10.
    #[test]
    fn section_5_3_gtin_plus_batch_lot() {
        let uri = "https://id.gs1.org/01/09520123456788/10/ABC123";
        let dl = DigitalLink::parse(uri).expect("GS1's own batch/lot example must parse");
        assert_eq!(dl.gtin().unwrap().as_str(), "09520123456788");
        assert_eq!(dl.batch(), Some("ABC123"));
        assert_eq!(dl.build(), uri);
    }

    /// §5.4 "GTIN + Serial Number" (SGTIN) — canonical form combined with AI 21.
    #[test]
    fn section_5_4_gtin_plus_serial() {
        let uri = "https://id.gs1.org/01/09520123456788/21/12345";
        let dl = DigitalLink::parse(uri).expect("GS1's own serial-number example must parse");
        assert_eq!(dl.gtin().unwrap().as_str(), "09520123456788");
        assert_eq!(dl.serial(), Some("12345"));
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
        assert_eq!(dl.gtin().unwrap().as_str(), "09520123456788");
        assert_eq!(dl.batch(), Some("ABC1"));
        assert_eq!(dl.serial(), Some("12345"));
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
