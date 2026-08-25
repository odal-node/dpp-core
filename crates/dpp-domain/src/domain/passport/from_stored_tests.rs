//! `Passport::from_stored`: reading a stored record, upcasting it where a lens
//! bridges the gap, and refusing where none does.

use super::*;
use crate::catalog::ProductGroupCatalog;
use crate::domain::product_group::{ProductGroup, ProductGroupData};
use crate::error::dpp::DppError;
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
