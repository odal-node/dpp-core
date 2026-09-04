//! `Passport::from_stored`: reading a stored record, upcasting it where a lens
//! bridges the gap, and refusing where none does.

use super::*;
use crate::catalog::ProductGroupCatalog;
use crate::error::dpp::DppError;
use crate::product_group::{ProductGroup, ProductGroupData};
use crate::schemas::lens::LensRegistry;

use super::tests::make_passport;

fn textile_passport() -> Passport {
    Passport {
        product_group: ProductGroup::Textile,
        product_group_data: Some(ProductGroupData::Textile(Box::new(
            crate::test_support::sample_textile_data(),
        ))),
        schema_version: "1.2.0".into(),
        ..make_passport()
    }
}

#[test]
fn from_stored_reads_current_shape_directly() {
    let passport = textile_passport();
    let doc = serde_json::to_value(&passport).expect("serialise");
    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();

    let back = Passport::from_stored(doc, &lenses, &catalog).expect("current shape reads as-is");
    assert_eq!(back.id, passport.id);
    assert_eq!(back.product_group_data, passport.product_group_data);
}

#[test]
fn from_stored_upcasts_a_legacy_country_field() {
    // A real textile 1.1.0 document: same schema, old country-of-origin key.
    // The 1.1.0 -> 1.2.0 lens exists for exactly this rename.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["schemaVersion"] = "1.1.0".into();
    let country = doc["productGroupData"]["countryOfOrigin"].take();
    doc["productGroupData"]["countryOfManufacturing"] = country;

    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();
    let back = Passport::from_stored(doc, &lenses, &catalog)
        .expect("the registered lens bridges 1.1.0 -> 1.2.0");

    let Some(ProductGroupData::Textile(textile)) = back.product_group_data else {
        panic!("expected textile product_group data");
    };
    assert_eq!(textile.country_of_origin, "PT");
}

#[test]
fn from_stored_refuses_a_gap_no_lens_bridges() {
    // A document recorded at a version no lens leaves must fail loudly and
    // typed — not panic, and not silently pass through as if it were current.
    //
    // The version is one this build has never served. Pointing this at a real
    // unbridged version instead couples a test about refusal semantics to the
    // registry's lens coverage, so legitimately bridging that version fails a
    // test that was never about it — which is what happened when textile
    // 1.0.0 -> 1.1.0 was added and this test still named 1.0.0.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["schemaVersion"] = "0.9.0".into();
    let country = doc["productGroupData"]["countryOfOrigin"].take();
    doc["productGroupData"]["countryOfManufacturing"] = country;

    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();
    let err = Passport::from_stored(doc, &lenses, &catalog)
        .expect_err("no lens chain reaches the current version from 0.9.0");
    assert!(
        matches!(err, DppError::SchemaIncompatible(_)),
        "expected a typed SchemaIncompatible refusal, got: {err}"
    );
}

#[test]
fn from_stored_surfaces_a_same_version_mismatch_as_serialisation() {
    // schemaVersion already matches current, so there is no version gap to
    // blame — a genuine shape mismatch, not a compatibility question.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["productGroupData"]
        .as_object_mut()
        .unwrap()
        .remove("gtin");

    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();
    let err = Passport::from_stored(doc, &lenses, &catalog)
        .expect_err("gtin is required and there is no version gap to bridge");
    assert!(
        matches!(err, DppError::Serialisation(_)),
        "expected a typed Serialisation error, got: {err}"
    );
}

/// A document carrying a removed envelope key is refused, not silently emptied.
///
/// This is the case the check exists for, and it is the one a plain deserialize
/// cannot catch: `Passport` sets no `deny_unknown_fields`, so serde ignores
/// `parentPassportRef` and hands back a passport whose `derived_from` is empty.
/// The control assertion below proves that is still what serde does — without
/// it, this test would keep passing against a build where the guard had been
/// deleted and something else happened to reject the document.
#[test]
fn from_stored_refuses_a_removed_envelope_key() {
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc.as_object_mut().expect("object").insert(
        "parentPassportRef".to_owned(),
        serde_json::json!({
            "uri": "https://id.example/dpp/predecessor",
            "publicJwsHash": "0".repeat(64),
        }),
    );

    // Control: serde alone accepts it and drops the edge.
    let lenient: Passport =
        serde_json::from_value(doc.clone()).expect("serde ignores unknown keys");
    assert!(
        lenient.derived_from.is_empty(),
        "control: a plain deserialize is expected to silently lose the edge"
    );

    let err = Passport::from_stored(doc, &LensRegistry::new(), &ProductGroupCatalog::new())
        .expect_err("a removed envelope key must be refused");

    match err {
        DppError::RemovedEnvelopeKey {
            removed,
            replacement,
        } => {
            assert_eq!(removed, "parentPassportRef");
            assert_eq!(replacement, "derivedFrom");
        }
        other => panic!("expected RemovedEnvelopeKey, got: {other:?}"),
    }
}

/// Every replacement named in `REMOVED_ENVELOPE_KEYS` must be a key the struct
/// actually emits, and every removed key must not be.
///
/// The list is strings on both sides. A replacement that no longer exists sends
/// an operator to a field that is not there, and a "removed" key that is in fact
/// still live would refuse every document that legitimately carries it.
#[test]
fn removed_envelope_keys_name_real_replacements() {
    assert!(
        !REMOVED_ENVELOPE_KEYS.is_empty(),
        "the list is permanent; entries are never pruned"
    );

    for &(removed, replacement) in REMOVED_ENVELOPE_KEYS {
        assert!(
            PASSPORT_WIRE_KEYS.contains(&replacement),
            "`{replacement}` is offered as the replacement for `{removed}` but is not a wire key"
        );
        assert!(
            !PASSPORT_WIRE_KEYS.contains(&removed),
            "`{removed}` is listed as removed but the struct still emits it"
        );
    }
}
