//! Percent-encoding and resolver path prefixes.

use super::super::*;

#[test]
fn percent_encoded_serial_decoded_on_parse() {
    // %2F is '/'
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN%2F001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.serial(), Some("SN/001"));
}

#[test]
fn slash_in_serial_encoded_on_build() {
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN1";
    let mut dl = DigitalLink::parse(uri).unwrap();
    dl.qualifiers = vec![("21".to_owned(), "SN/001".to_owned())];
    assert!(dl.build().contains("/21/SN%2F001"));
}

#[test]
fn question_mark_in_serial_encoded_on_build() {
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN1";
    let mut dl = DigitalLink::parse(uri).unwrap();
    dl.qualifiers = vec![("21".to_owned(), "SN?001".to_owned())];
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
