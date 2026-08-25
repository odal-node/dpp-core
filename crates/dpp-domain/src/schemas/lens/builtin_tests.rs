//! Behaviour of the compiled-in lenses: what each one derives, and what it refuses.

use serde_json::Value;

use super::tests::{battery_v1, v};
use super::*;
use crate::schemas::VersionedSchemaRegistry;

#[test]
fn battery_v1_upcasts_to_v2_and_validates() {
    let lenses = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let original = battery_v1();

    let derived = lenses
        .upcast("battery", &original, &v("1.0.0"), &v("2.0.0"))
        .unwrap();

    // The derived view is honest about its provenance.
    assert!(derived.derived);
    assert!(!derived.lossy);
    assert_eq!(derived.from, "1.0.0");
    assert_eq!(derived.to, "2.0.0");
    assert_eq!(
        derived.lens_chain,
        vec![["1.0.0".to_string(), "2.0.0".to_string()]]
    );

    // The real transform ran: Wh derived from kWh.
    assert_eq!(derived.data["ratedEnergyWh"].as_f64(), Some(4800.0));

    // And the derived data validates against the v2 schema.
    schemas
        .validate("battery", &v("2.0.0"), &derived.data)
        .expect("derived view must validate against v2");

    // The original is untouched (lens clones its input).
    assert!(original.get("ratedEnergyWh").is_none());
}

/// A minimal but valid v1.0.0 steel record (schema-required fields).
fn steel_v1() -> Value {
    serde_json::json!({
        "gtin": "09506000134352",
        "co2ePerTonneSteel": 1.8,
        "recycledScrapContentPct": 35.0,
        "productCategory": "flat",
        "countryOfProduction": "DE",
        "productionRoute": "electric-arc"
    })
}

#[test]
fn steel_v1_upcasts_to_v1_1_and_renames_country_field() {
    let lenses = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let original = steel_v1();

    let derived = lenses
        .upcast("steel", &original, &v("1.0.0"), &v("1.1.0"))
        .unwrap();

    assert!(!derived.lossy);
    assert_eq!(derived.data["countryOfOrigin"], "DE");
    assert!(derived.data.get("countryOfProduction").is_none());

    schemas
        .validate("steel", &v("1.1.0"), &derived.data)
        .expect("derived view must validate against v1.1.0");

    // The original is untouched (lens clones its input).
    assert_eq!(original["countryOfProduction"], "DE");
}

/// A minimal but valid v1.1.0 textile record (schema-required fields).
fn textile_v1_1() -> Value {
    serde_json::json!({
        "gtin": "09506000134352",
        "fibreComposition": [{"fibre": "cotton", "pct": 100.0}],
        "countryOfManufacturing": "PT",
        "careInstructions": "Hand wash",
        "chemicalComplianceStandard": "REACH"
    })
}

#[test]
fn textile_v1_1_upcasts_to_v1_2_and_renames_country_field() {
    let lenses = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let original = textile_v1_1();

    let derived = lenses
        .upcast("textile", &original, &v("1.1.0"), &v("1.2.0"))
        .unwrap();

    assert!(!derived.lossy);
    assert_eq!(derived.data["countryOfOrigin"], "PT");
    assert!(derived.data.get("countryOfManufacturing").is_none());

    schemas
        .validate("textile", &v("1.2.0"), &derived.data)
        .expect("derived view must validate against v1.2.0");
}

#[test]
fn battery_lens_derives_clean_watt_hours() {
    // Correct Wh regardless of f64 noise, and fractional Wh is preserved
    // (not rounded to whole Wh) — distinguishing "strip noise" from "round".
    let reg = LensRegistry::new();
    for (kwh, wh) in [(4.8, 4800.0), (0.1, 100.0), (4.8005, 4800.5)] {
        let mut data = battery_v1();
        data.as_object_mut()
            .unwrap()
            .insert("ratedCapacityKwh".into(), serde_json::json!(kwh));
        let d = reg
            .upcast("battery", &data, &v("1.0.0"), &v("2.0.0"))
            .unwrap();
        assert_eq!(d.data["ratedEnergyWh"].as_f64(), Some(wh), "kwh {kwh}");
    }
}

#[test]
fn battery_v2_4_to_v2_5_passes_through_with_battery_type() {
    let lenses = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let mut v2_4 = battery_v1();
    v2_4.as_object_mut()
        .unwrap()
        .insert("batteryType".into(), serde_json::json!("ev"));

    let derived = lenses
        .upcast("battery", &v2_4, &v("2.4.0"), &v("2.5.0"))
        .unwrap();

    assert!(!derived.lossy);
    assert_eq!(derived.data["batteryType"], "ev");
    schemas
        .validate("battery", &v("2.5.0"), &derived.data)
        .expect("derived view must validate against v2.5.0");
}

#[test]
fn battery_v2_4_to_v2_5_refuses_when_battery_type_is_absent() {
    // The one hard question this lens exists to answer: a passport
    // published without batteryType predates the v2.5.0 mandate and
    // cannot be upgraded into satisfying it without inventing a value.
    // A typed refusal is correct here, not a silent identity or a guess.
    let lenses = LensRegistry::new();
    let err = lenses
        .upcast("battery", &battery_v1(), &v("2.4.0"), &v("2.5.0"))
        .unwrap_err();
    assert!(matches!(err, UpcastError::Transform(_)));
}

/// A minimal but valid v1.1.0 electronics record (schema-required fields).
fn electronics_v1_1(product_category: &str) -> Value {
    serde_json::json!({
        "gtin": "09506000134352",
        "productCategory": product_category,
        "energyEfficiencyClass": "B",
        "co2ePerUnitKg": 120.0
    })
}

#[test]
fn electronics_v1_1_to_v1_2_passes_through_a_surviving_category() {
    let lenses = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let v1_1 = electronics_v1_1("smartphone");

    let derived = lenses
        .upcast("electronics", &v1_1, &v("1.1.0"), &v("1.2.0"))
        .unwrap();

    assert!(!derived.lossy);
    assert_eq!(derived.data["productCategory"], "smartphone");
    schemas
        .validate("electronics", &v("1.2.0"), &derived.data)
        .expect("derived view must validate against v1.2.0");
}

#[test]
fn electronics_v1_1_to_v1_2_refuses_a_removed_category() {
    // "laptop" was schema-valid at v1.1.0 but has no lawful basis under
    // Regulation (EU) 2023/1670 Art. 1(1) — there is no v1.2.0 value to
    // substitute that would not misdescribe the product.
    let lenses = LensRegistry::new();
    let err = lenses
        .upcast(
            "electronics",
            &electronics_v1_1("laptop"),
            &v("1.1.0"),
            &v("1.2.0"),
        )
        .unwrap_err();
    assert!(matches!(err, UpcastError::Transform(_)));
}

#[test]
fn battery_v2_5_to_v2_6_upgrades_a_record_with_no_cycle_count() {
    // The v2.6.0 relaxation exists for industrial batteries whose lifetime
    // cannot be expressed in cycles (Annex XIII point 1(j)). A record with
    // no expectedLifetimeCycles fails v2.5.0 and must pass v2.6.0 — this is
    // the opposite of the v2.4.0 hop above, which refuses. A relaxation
    // cannot strand a document; a new obligation can.
    let lenses = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let mut data = battery_v1();
    let obj = data.as_object_mut().unwrap();
    obj.insert("batteryType".into(), serde_json::json!("industrial"));
    obj.remove("expectedLifetimeCycles");

    assert!(
        schemas.validate("battery", &v("2.5.0"), &data).is_err(),
        "the fixture must be one v2.5.0 rejects, or this proves nothing"
    );

    let derived = lenses
        .upcast("battery", &data, &v("2.5.0"), &v("2.6.0"))
        .unwrap();
    assert!(!derived.lossy);
    schemas
        .validate("battery", &v("2.6.0"), &derived.data)
        .expect("derived view must validate against v2.6.0");
}

#[test]
fn battery_v2_5_to_v2_6_does_not_invent_an_empty_point_4_block() {
    // Absence stays absent. Materialising an empty dynamicPerformance would
    // assert that the battery reported measured values it never reported.
    let lenses = LensRegistry::new();
    let mut data = battery_v1();
    data.as_object_mut()
        .unwrap()
        .insert("batteryType".into(), serde_json::json!("ev"));

    let derived = lenses
        .upcast("battery", &data, &v("2.5.0"), &v("2.6.0"))
        .unwrap();

    for key in ["dynamicPerformance", "batteryStatus", "usageHistory"] {
        assert!(
            derived.data.get(key).is_none(),
            "the lens materialised '{key}', which the source never carried"
        );
    }
}

#[test]
fn battery_v2_5_to_v2_6_carries_both_legacy_keys_verbatim() {
    // Neither legacy value can be assigned to a half of its Annex XIII pair
    // — one names a condition the annex does not state, the other cannot
    // say whether it was cell or pack. So both survive under their own
    // names rather than being moved, guessed at, or refused.
    let lenses = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let mut data = battery_v1();
    let obj = data.as_object_mut().unwrap();
    obj.insert("batteryType".into(), serde_json::json!("ev"));
    obj.insert("roundTripEfficiencyPct".into(), serde_json::json!(91.5));
    obj.insert("internalResistanceMohm".into(), serde_json::json!(12.0));

    let derived = lenses
        .upcast("battery", &data, &v("2.5.0"), &v("2.6.0"))
        .unwrap();

    assert!(!derived.lossy);
    assert_eq!(derived.data["roundTripEfficiencyPct"], 91.5);
    assert_eq!(derived.data["internalResistanceMohm"], 12.0);
    for successor in [
        "roundTripEfficiencyAtHalfCycleLifePct",
        "initialRoundTripEfficiencyPct",
        "internalCellResistanceMohm",
        "internalPackResistanceMohm",
    ] {
        assert!(
            derived.data.get(successor).is_none(),
            "the lens populated '{successor}', inventing a distinction the                  source never made"
        );
    }
    schemas
        .validate("battery", &v("2.6.0"), &derived.data)
        .expect("both legacy keys validate against v2.6.0");
}

#[test]
fn battery_v2_6_accepts_the_individual_battery_tier() {
    let schemas = VersionedSchemaRegistry::new();
    let mut data = battery_v1();
    let obj = data.as_object_mut().unwrap();
    obj.insert("batteryType".into(), serde_json::json!("ev"));
    obj.insert(
        "dynamicPerformance".into(),
        serde_json::json!({ "ratedCapacityAh": 92.0, "capacityFadePct": 8.0 }),
    );
    obj.insert("batteryStatus".into(), serde_json::json!("repurposed"));
    obj.insert(
        "usageHistory".into(),
        serde_json::json!({
            "chargeDischargeCycles": 412,
            "stateOfCharge": [
                { "recordedAt": "2026-08-11T09:00:00Z", "stateOfChargePct": 61.5 }
            ]
        }),
    );

    schemas
        .validate("battery", &v("2.6.0"), &data)
        .expect("the point 4 tier validates");
}

#[test]
fn battery_v2_6_refuses_a_status_the_annex_does_not_enumerate() {
    // Annex XIII point 4(c) spells the set out inline, so it is closed for
    // the same reason batteryType is.
    let schemas = VersionedSchemaRegistry::new();
    let mut data = battery_v1();
    let obj = data.as_object_mut().unwrap();
    obj.insert("batteryType".into(), serde_json::json!("ev"));
    obj.insert("batteryStatus".into(), serde_json::json!("refurbished"));

    assert!(
        schemas.validate("battery", &v("2.6.0"), &data).is_err(),
        "'refurbished' is not one of the five the annex names"
    );
}

#[test]
fn battery_lens_with_nothing_to_derive_still_validates_against_v2() {
    // A v1 record with no ratedCapacityKwh: the lens has nothing to derive,
    // and the result must still validate against v2 (all v2 additions optional).
    let mut data = battery_v1();
    data.as_object_mut().unwrap().remove("ratedCapacityKwh");
    let reg = LensRegistry::new();
    let schemas = VersionedSchemaRegistry::new();
    let derived = reg
        .upcast("battery", &data, &v("1.0.0"), &v("2.0.0"))
        .unwrap();
    assert!(derived.data.get("ratedEnergyWh").is_none());
    schemas
        .validate("battery", &v("2.0.0"), &derived.data)
        .expect("a v1 record with no rated capacity still validates against v2");
}
