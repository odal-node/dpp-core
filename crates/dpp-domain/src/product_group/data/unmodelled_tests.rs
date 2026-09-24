//! What an untyped payload answers, and what it keeps.

use serde_json::json;

use super::ProductGroupData;
use crate::product_group::ProductGroup;

fn untyped(product_identifier: serde_json::Value) -> serde_json::Value {
    json!({
        "productGroup": "photovoltaic",
        "productIdentifier": product_identifier,
        "moduleEfficiencyPct": 22.4,
    })
}

/// An identifier's key and shape are fixed by EN 18219 clause 5, not by any one
/// act, so an untyped payload answers with the one it carries — through
/// deserialisation and through `other` alike.
#[test]
fn an_untyped_payload_answers_the_identifier_it_carries() {
    let wire = untyped(json!({ "scheme": "gs1", "gtin": "09506000134352" }));

    let deserialised: ProductGroupData = serde_json::from_value(wire.clone()).expect("untyped");
    let constructed = ProductGroupData::other(wire).expect("untyped");

    for data in [&deserialised, &constructed] {
        assert_eq!(
            data.product_group(),
            ProductGroup::Other("photovoltaic".into())
        );
        let identifier = data.product_identifier().expect("carried");
        assert_eq!(identifier.as_str(), "09506000134352");
        assert_eq!(data.gtin(), Some("09506000134352"));
    }
    assert_eq!(deserialised, constructed);
}

/// A self-issuing scheme is read too: it identifies a product and has no GTIN,
/// the same two answers a typed payload gives for it.
#[test]
fn an_untyped_payload_carrying_a_did_answers_with_no_gtin() {
    let data = ProductGroupData::other(untyped(json!({
        "scheme": "did",
        "did": "did:web:example.com:p:1",
    })))
    .expect("untyped");

    assert!(data.product_identifier().is_some());
    assert_eq!(data.gtin(), None);
}

/// 🚨 A malformed identifier must not fail the payload. An untyped group has
/// to round-trip whatever it carries — a fetched passport cannot be rewritten —
/// so the value is kept verbatim and simply answers no identifier.
#[test]
fn a_malformed_identifier_is_kept_verbatim_and_answers_none() {
    for malformed in [
        json!({ "scheme": "gs1", "gtin": "09506000134351" }),
        json!({ "scheme": "nonsense", "value": "x" }),
        json!("09506000134352"),
        json!(null),
    ] {
        let wire = untyped(malformed.clone());
        let data: ProductGroupData = serde_json::from_value(wire.clone())
            .unwrap_or_else(|e| panic!("{malformed} must not fail the payload: {e}"));

        assert_eq!(data.product_identifier(), None, "{malformed}");
        assert_eq!(serde_json::to_value(&data).expect("serialises"), wire);
    }
}

/// No `productIdentifier` at all is the plain absence.
#[test]
fn an_untyped_payload_without_an_identifier_answers_none() {
    let data = ProductGroupData::other(json!({ "productGroup": "photovoltaic" })).expect("untyped");
    assert_eq!(data.product_identifier(), None);
}

/// The questions only an act can define — which key is the model identifier,
/// the category, the SVHC declaration — have no answer in an untyped object.
#[test]
fn an_untyped_payload_answers_nothing_only_an_act_defines() {
    let data = ProductGroupData::other(json!({
        "productGroup": "photovoltaic",
        "modelIdentifier": "PV-400",
        "productCategory": "module",
        "svhcSubstances": [],
    }))
    .expect("untyped");

    assert_eq!(data.model_identifier(), None);
    assert_eq!(data.product_category(), None);
    assert_eq!(data.svhc_substances(), None);
}
