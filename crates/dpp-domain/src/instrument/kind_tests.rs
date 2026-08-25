//! Which Treaty instrument a catalog entry is, and how each kind round-trips.

use super::*;

#[test]
fn every_modelled_variant_round_trips_as_a_bare_string() {
    for kind in [
        InstrumentKind::Framework,
        InstrumentKind::Delegated,
        InstrumentKind::Direct,
        InstrumentKind::Adjacent,
        InstrumentKind::Implementing,
    ] {
        let json = serde_json::to_string(&kind).expect("serialise");
        assert!(
            json.starts_with('"'),
            "{kind:?} must render as a string, got {json}"
        );
        assert_eq!(
            serde_json::from_str::<InstrumentKind>(&json).expect("deserialise"),
            kind
        );
    }
}

#[test]
fn an_unmodelled_kind_is_absorbed_not_rejected() {
    let parsed: InstrumentKind =
        serde_json::from_str("\"international-agreement\"").expect("must not fail");
    assert_eq!(
        parsed,
        InstrumentKind::Other("international-agreement".to_owned())
    );
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        "\"international-agreement\""
    );
}
