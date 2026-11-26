//! What a Digital Link must refuse.

use super::super::*;

#[test]
fn unknown_ai_rejected() {
    // `04` is unassigned in GS1's dictionary. `99` is not — it is INTERNAL, and
    // the hand-written table used to refuse it, which is the quiet failure:
    // refusing a link a conformant partner may legitimately send.
    let uri = "https://id.odal-node.io/01/09506000134352/04/unknown/21/SN1";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::UnknownApplicationIdentifier(_))
    ));
}

#[test]
fn a_data_attribute_in_the_path_is_rejected() {
    // AI 99 (INTERNAL) is a real GS1 AI, so this is not "unknown". But GS1's
    // Digital Link grammar puts only the primary key and its qualifiers in the
    // path — a data attribute belongs in the query string. Adjudicated by the
    // GS1 Barcode Syntax Engine, which rejected this URI when an earlier
    // revision of the parser accepted it.
    let uri = "https://id.odal-node.io/01/09506000134352/21/SN1/99/INTERNALDATA";
    assert!(matches!(
        DigitalLink::parse(uri),
        Err(DigitalLinkError::DataAttributeInPath(_))
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
