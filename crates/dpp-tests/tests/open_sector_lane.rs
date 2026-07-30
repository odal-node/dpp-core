//! The sector axis is open: a product group this build has no typed variant
//! for still round-trips, keyed by its own name.
//!
//! Before this, `Sector` and `SectorData` were plain derives — `SectorData`
//! internally tagged on `sector` — so an unrecognised tag failed to
//! deserialize outright, and the `Other` variant matched only the literal
//! string `"other"`, discarding the real key. The catalog, the schema registry
//! and the plugin manifests were all string-keyed data, and the wire was the
//! one closed part: adding a product group meant releasing this crate.
//!
//! These tests pin the property that makes it data instead.

use dpp_domain::{Sector, SectorData};
use serde_json::json;

/// The tag of a sector deliberately absent from this build's catalog.
const UNKNOWN: &str = "packaging";

#[test]
fn an_unknown_sector_tag_deserializes_instead_of_failing() {
    let wire = json!({
        "sector": UNKNOWN,
        "materialFamily": "corrugated-fibreboard",
        "recycledContentPct": 82.5,
    });

    let data: SectorData = serde_json::from_value(wire)
        .expect("an unknown sector must not be a deserialization error");

    match &data {
        SectorData::Other { sector, .. } => assert_eq!(sector, UNKNOWN),
        other => panic!("expected an untyped payload, got {other:?}"),
    }
}

#[test]
fn an_unknown_sector_keeps_its_identity_through_a_round_trip() {
    let wire = json!({
        "sector": UNKNOWN,
        "materialFamily": "corrugated-fibreboard",
        "recycledContentPct": 82.5,
    });

    let data: SectorData = serde_json::from_value(wire.clone()).expect("deserializes");
    let back = serde_json::to_value(&data).expect("serializes");

    // Byte-identical, not merely "an object with the right tag": a passport
    // issued for a sector this build cannot type must survive storage and
    // retrieval unchanged, or the signature over it stops verifying.
    assert_eq!(
        back, wire,
        "an untyped sector payload was altered by a round trip"
    );
}

#[test]
fn the_sector_discriminant_carries_the_unknown_key() {
    let data: SectorData =
        serde_json::from_value(json!({ "sector": UNKNOWN })).expect("deserializes");

    // `sector()` must report the real key, not a placeholder — the catalog,
    // schema registry and plugin host all dispatch on it.
    let sector = data.sector();
    assert_eq!(sector.catalog_key(), UNKNOWN);
    assert_eq!(sector, Sector::Other(UNKNOWN.to_owned()));
}

#[test]
fn the_sector_enum_round_trips_an_unknown_tag() {
    let sector: Sector = serde_json::from_value(json!(UNKNOWN)).expect("deserializes");
    assert_eq!(sector, Sector::Other(UNKNOWN.to_owned()));
    assert_eq!(sector.wire_str(), UNKNOWN);
    assert_eq!(serde_json::to_value(&sector).unwrap(), json!(UNKNOWN));
}

#[test]
fn two_unknown_sectors_stay_distinct() {
    // The old `Other` collapsed every unrecognised sector to one value, so two
    // different product groups became indistinguishable after a round trip.
    let a: SectorData = serde_json::from_value(json!({ "sector": "packaging" })).unwrap();
    let b: SectorData = serde_json::from_value(json!({ "sector": "automotive" })).unwrap();

    assert_ne!(a.sector(), b.sector());
    assert_eq!(a.sector().catalog_key(), "packaging");
    assert_eq!(b.sector().catalog_key(), "automotive");
}

#[test]
fn known_sectors_still_deserialize_to_their_typed_variants() {
    // The open lane must not cost the typed lane: an in-force sector still
    // lands on its struct, not in the untyped fallback.
    let battery = json!({
        "sector": "battery",
        "gtin": "09506000134352",
        "batteryChemistry": "lfp",
        "nominalVoltageV": 3.2,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
    });

    let data: SectorData = serde_json::from_value(battery).expect("battery deserializes");
    assert!(
        matches!(data, SectorData::Battery(_)),
        "a known sector fell through to the untyped variant: {data:?}"
    );
    assert_eq!(data.sector(), Sector::Battery);
}

#[test]
fn the_kebab_case_catalog_spelling_resolves_to_the_typed_variant() {
    // Two spellings are in circulation — the catalog's kebab-case key and the
    // camelCase wire tag. Both name one product group, and neither may fall
    // through to the untyped lane.
    assert_eq!(Sector::from_wire_tag("unsold-goods"), Sector::UnsoldGoods);
    assert_eq!(Sector::from_wire_tag("unsoldGoods"), Sector::UnsoldGoods);
}

#[test]
fn a_payload_with_no_sector_tag_is_still_an_error() {
    // Opening the lane must not open it so far that an untagged payload is
    // accepted: with no tag there is no identity to preserve.
    let result: Result<SectorData, _> = serde_json::from_value(json!({ "someField": 1 }));
    assert!(
        result.is_err(),
        "a payload with no sector tag must be refused"
    );
}
