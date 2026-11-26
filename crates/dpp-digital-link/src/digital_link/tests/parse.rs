//! Parsing a Digital Link and building it back.

use super::super::*;

#[test]
fn round_trip_gtin_serial() {
    let uri = "https://id.odal-node.io/01/09506000134352/21/ABC123";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.gtin().unwrap().as_str(), "09506000134352");
    assert_eq!(dl.serial(), Some("ABC123"));
    assert_eq!(dl.build(), uri);
}

#[test]
fn a_scheme_other_than_http_or_https_is_rejected() {
    assert!(matches!(
        DigitalLink::parse("ftp://id.odal-node.io/01/09506000134352"),
        Err(DigitalLinkError::InvalidScheme(_))
    ));
}

#[test]
fn round_trips_the_first_qualifier_sequence() {
    // 22 (variant) → 10 (batch) → 21 (serial): the first of the two alternative
    // sequences AI 01 declares as `dlpkey=22,10,21|235`.
    let uri = "https://id.odal-node.io/01/09506000134352/22/VAR-1/10/LOT-9/21/SN-7";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.variant(), Some("VAR-1"));
    assert_eq!(dl.batch(), Some("LOT-9"));
    assert_eq!(dl.serial(), Some("SN-7"));
    assert_eq!(dl.tpcsn(), None);
    assert_eq!(dl.build(), uri);
}

#[test]
fn round_trips_the_alternative_qualifier_sequence() {
    // 235 is the *other* alternative, used on its own.
    let uri = "https://id.odal-node.io/01/09506000134352/235/TPX-3";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.tpcsn(), Some("TPX-3"));
    assert_eq!(dl.serial(), None);
    assert_eq!(dl.build(), uri);
}

#[test]
fn qualifiers_from_two_alternative_sequences_are_rejected() {
    // GS1 writes AI 01's qualifiers as `dlpkey=22,10,21|235`, and the
    // dictionary's own header defines `|` as *alternative* sequences: a path
    // uses "22,10,21" or "235", never both. A hand-written table that ordered
    // them 22→1, 10→2, 21→3, 235→4 turned two alternatives into one ladder and
    // accepted a link GS1 does not define.
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN-7/235/TPX-3";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::MixedQualifierSequences { .. })
    ));
}

#[test]
fn parse_with_batch() {
    let uri = "https://id.odal-node.io/01/09506000134352/10/BATCH01/21/SN001";
    let dl = DigitalLink::parse(uri).unwrap();
    assert_eq!(dl.batch(), Some("BATCH01"));
    assert_eq!(dl.serial(), Some("SN001"));
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
    // AI 21 is a qualifier, not a primary key, so there is genuinely no primary
    // key here and `MissingGtin` is the accurate verdict.
    let uri = "https://id.odal-node.io/21/SN1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::MissingGtin)
    ));
}
