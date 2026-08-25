//! Serde round-trips, wire values, and the enum metadata each product group
//! declares.

use super::*;
use crate::domain::gtin::Gtin;

use super::tests::test_textile_data;

// ── Serde round-trips ─────────────────────────────────────────────────

#[test]
fn product_group_data_battery_round_trip() {
    let data = ProductGroupData::Battery(Box::new(BatteryData {
        recycled_content_lithium_pct: Some(12.5),
        rated_capacity_kwh: Some(32.0),
        carbon_footprint_class: Some(CarbonFootprintClass::new("B").expect("valid label")),
        carbon_footprint_class_ruleset_id: Some("test-cfb-classes".into()),
        carbon_footprint_class_ruleset_version: Some("0.0.0-test".into()),
        ..crate::test_support::sample_battery_data()
    }));
    let json = serde_json::to_value(&data).unwrap();
    assert_eq!(
        json["productGroup"], "battery",
        "product_group tag must be lowercase"
    );
    assert_eq!(json["batteryChemistry"], "LFP");
    assert_eq!(json["gtin"], "09506000134352");
    let back: ProductGroupData = serde_json::from_value(json).unwrap();
    assert_eq!(data, back);
}

// Regression: every BatteryType and BatteryChemistry variant must serialise to
// a wire value the JSON schema accepts. The Sli "sli" vs schema
// "starting-lighting-ignition" mismatch was silent without this.
#[test]
fn battery_enum_wire_values_match_schema() {
    let cases: &[(BatteryType, &str)] = &[
        (BatteryType::Portable, "portable"),
        (BatteryType::Industrial, "industrial"),
        (BatteryType::Ev, "ev"),
        (BatteryType::Lmt, "lmt"),
        (BatteryType::Sli, "starting-lighting-ignition"),
    ];
    for (variant, expected) in cases {
        let json = serde_json::to_value(variant).unwrap();
        assert_eq!(
            json.as_str().unwrap(),
            *expected,
            "BatteryType::{variant:?} must serialise as \"{expected}\""
        );
        let back: BatteryType = serde_json::from_str(&format!("\"{expected}\"")).unwrap();
        assert_eq!(back, *variant, "round-trip failed for \"{expected}\"");
    }

    let chem_cases: &[(BatteryChemistry, &str)] = &[
        (BatteryChemistry::Lfp, "LFP"),
        (BatteryChemistry::Nmc, "NMC"),
        (BatteryChemistry::Nca, "NCA"),
        (BatteryChemistry::Lco, "LCO"),
        (BatteryChemistry::NiMh, "NiMH"),
        (BatteryChemistry::NiCd, "NiCd"),
        (BatteryChemistry::LeadAcid, "lead-acid"),
        (BatteryChemistry::SolidState, "solid-state"),
    ];
    for (variant, expected) in chem_cases {
        let json = serde_json::to_value(variant).unwrap();
        assert_eq!(
            json.as_str().unwrap(),
            *expected,
            "BatteryChemistry::{variant:?} must serialise as \"{expected}\""
        );
    }
}

// Regression: Art. 1(3) is a closed set of five categories, and an
// unrecognised value used to flatten to `BatteryType::Other`, discarding the
// declared string on round-trip — the same defect class already fixed for
// `CarbonFootprintClass`. There is no catch-all left to absorb it.
#[test]
fn unrecognised_battery_type_is_rejected_not_flattened() {
    for bad in ["\"stationary\"", "\"Portable\"", "\"\"", "null"] {
        assert!(
            serde_json::from_str::<BatteryType>(bad).is_err(),
            "should reject {bad}"
        );
    }
}

// Regression: Art. 1(1) is a closed set of four device types, and the
// removed values (`laptop`, `tv`, etc.) must be rejected outright rather
// than accepted by an `Other`/`#[serde(other)]` catch-all this type does not
// carry.
#[test]
fn device_type_wire_values_round_trip() {
    let cases: &[(DeviceType, &str)] = &[
        (DeviceType::Smartphone, "smartphone"),
        (DeviceType::OtherMobilePhone, "other-mobile-phone"),
        (DeviceType::CordlessPhone, "cordless-phone"),
        (DeviceType::Tablet, "tablet"),
    ];
    for (variant, expected) in cases {
        let json = serde_json::to_value(variant).unwrap();
        assert_eq!(
            json.as_str().unwrap(),
            *expected,
            "DeviceType::{variant:?} must serialise as \"{expected}\""
        );
        let back: DeviceType = serde_json::from_str(&format!("\"{expected}\"")).unwrap();
        assert_eq!(back, *variant, "round-trip failed for \"{expected}\"");
    }
}

#[test]
fn unrecognised_device_type_is_rejected() {
    for bad in ["\"laptop\"", "\"tv\"", "\"Smartphone\"", "\"\"", "null"] {
        assert!(
            serde_json::from_str::<DeviceType>(bad).is_err(),
            "should reject {bad}"
        );
    }
}

#[test]
fn product_group_data_textile_round_trip() {
    let mut data = test_textile_data();
    data.fibre_composition = vec![FibreEntry {
        fibre: "cotton".into(),
        pct: 100.0,
        country_of_origin: Some("IN".into()),
    }];
    data.repair_score = Some(6.0);
    data.carbon_footprint_kg_co2e = Some(8.5);
    data.country_of_raw_material_origin = Some("IN".into());
    data.durability_score = Some(7.5);
    data.microplastic_shedding_mg_per_wash = Some(12.3);
    data.svhc_substances = Some(vec![SvhcSubstance {
        cas_number: "80-05-7".into(),
        substance_name: "Bisphenol A".into(),
        concentration_pct: 0.15,
        location_in_product: Some("coating".into()),
        scip_notification_id: Some("SCIP-12345".into()),
    }]);

    let product_group = ProductGroupData::Textile(Box::new(data.clone()));
    let json = serde_json::to_value(&product_group).unwrap();
    assert_eq!(
        json["productGroup"], "textile",
        "product_group tag must be lowercase"
    );
    assert_eq!(json["countryOfOrigin"], "BD");
    assert_eq!(json["durabilityScore"], 7.5);
    assert_eq!(json["microplasticSheddingMgPerWash"], 12.3);
    assert!(json["svhcSubstances"].is_array());
    assert_eq!(json["svhcSubstances"][0]["casNumber"], "80-05-7");
    assert_eq!(
        json["fibreComposition"][0]["countryOfOrigin"], "IN",
        "per-fibre origin must serialize"
    );

    let back: ProductGroupData = serde_json::from_value(json).unwrap();
    assert_eq!(ProductGroupData::Textile(Box::new(data)), back);
}

#[test]
fn textile_none_fields_not_serialized() {
    // Verify skip_serializing_if works — None fields should be absent from JSON
    let data = ProductGroupData::Textile(Box::new(test_textile_data()));
    let json = serde_json::to_value(&data).unwrap();
    assert!(
        json.get("svhcSubstances").is_none(),
        "None svhc should be absent"
    );
    assert!(
        json.get("durabilityScore").is_none(),
        "None durability should be absent"
    );
    assert!(json.get("disassemblyInstructions").is_none());
    assert!(json.get("microplasticSheddingMgPerWash").is_none());
}

#[test]
fn textile_v1_data_deserializes_with_defaults() {
    // Minimal JSON (only required fields, current wire shape) must still
    // deserialize into the expanded struct with every optional field defaulted.
    let v1_json = serde_json::json!({
        "productGroup": "textile",
        "gtin": "09506000134352",
        "fibreComposition": [{"fibre": "cotton", "pct": 100.0}],
        "countryOfOrigin": "PT",
        "careInstructions": "Hand wash",
        "chemicalComplianceStandard": "REACH"
    });
    let parsed: ProductGroupData = serde_json::from_value(v1_json).unwrap();
    if let ProductGroupData::Textile(t) = parsed {
        assert_eq!(t.country_of_origin, "PT");
        assert!(t.svhc_substances.is_none());
        assert!(t.durability_score.is_none());
        assert!(t.microplastic_shedding_mg_per_wash.is_none());
        assert!(t.fibre_composition[0].country_of_origin.is_none());
    } else {
        panic!("expected Textile variant");
    }
}

// ── ProductGroup enum metadata ──────────────────────────────────────────────

#[test]
fn every_product_group_declares_a_catalog_key() {
    // Every variant's catalog_key() must be total — every match arm exercised.
    // Retention is deliberately absent here: it lives on the binding between an
    // act and a product group, and `InstrumentCatalog::retention_for` is its only
    // accessor.
    let all = [
        (ProductGroup::Battery, "battery"),
        (ProductGroup::Textile, "textile"),
        (ProductGroup::UnsoldGoods, "unsold-goods"),
        (ProductGroup::Steel, "steel"),
        (ProductGroup::Electronics, "electronics"),
        (ProductGroup::Construction, "construction"),
        (ProductGroup::Tyre, "tyre"),
        (ProductGroup::Toy, "toy"),
        (ProductGroup::Aluminium, "aluminium"),
        (ProductGroup::Furniture, "furniture"),
        (ProductGroup::Detergent, "detergent"),
        (ProductGroup::Other("other".into()), "other"),
    ];
    for (product_group, key) in all {
        assert_eq!(product_group.catalog_key(), key);
    }
}

#[test]
fn product_group_discriminant_matches_variant() {
    let battery = ProductGroupData::Battery(Box::new(BatteryData {
        gtin: Gtin::parse("00000000000000").unwrap(),
        battery_chemistry: BatteryChemistry::Nmc,
        nominal_voltage_v: 4.0,
        nominal_capacity_ah: 50.0,
        expected_lifetime_cycles: Some(1000),
        co2e_per_unit_kg: 40.0,
        ..crate::test_support::sample_battery_data()
    }));
    assert_eq!(battery.product_group(), ProductGroup::Battery);
}
