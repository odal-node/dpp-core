//! Query strings and the normalisation of a GTIN.

use super::super::*;

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
    assert_eq!(dl.serial(), Some("SN001"));
}

#[test]
fn query_only_uri_returns_gtin_cleanly() {
    let uri = "https://id.odal-node.io/01/09506000134352?linkType=gs1:pip";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin().unwrap().as_str(), "09506000134352");
    assert_eq!(dl.serial(), None);
}

// ── 3.4a: GTIN normalisation ──────────────────────────────────

#[test]
fn gtin_13_normalised_to_14() {
    // EAN-13 5901234123457 → GTIN-14 05901234123457 (check digit preserved)
    let uri = "https://id.odal-node.io/01/5901234123457/21/SN1";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin().unwrap().as_str(), "05901234123457");
}

#[test]
fn gtin_12_normalised_to_14() {
    // UPC-A 012345678905 → GTIN-14 00012345678905
    let uri = "https://id.odal-node.io/01/012345678905/21/SN1";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin().unwrap().as_str(), "00012345678905");
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
