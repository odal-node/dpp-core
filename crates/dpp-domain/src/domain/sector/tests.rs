//! Redaction, validation, and serde round-trip tests for sector data.

use super::*;
use crate::catalog::{RegulatoryStatus, SectorDescriptor};
use crate::domain::gtin::Gtin;
use crate::domain::identity::AccessTier;

// ── redact_sector_data ────────────────────────────────────────────────

fn battery_descriptor_with_tiers() -> SectorDescriptor {
    use std::collections::HashMap;
    let mut access_tiers = HashMap::new();
    access_tiers.insert("dueDiligenceUrl".into(), AccessTier::Professional);
    access_tiers.insert("criticalRawMaterials".into(), AccessTier::Professional);
    access_tiers.insert(
        "disassemblyInstructionsUrl".into(),
        AccessTier::Professional,
    );
    SectorDescriptor {
        key: "battery".into(),
        title: "Battery".into(),
        status: RegulatoryStatus::InForce,
        legal_basis: vec!["EU 2023/1542".into()],
        dpp_applies_from: None,
        retention_years: 10,
        schema_versions: vec!["2.0.0".into()],
        current_schema_version: "2.0.0".into(),
        product_categories: vec![],
        access_tiers,
        plugin: None,
        notes: None,
    }
}

fn minimal_battery_data() -> SectorData {
    SectorData::Battery(BatteryData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: 3000,
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: None,
        recycled_content_nickel_pct: None,
        state_of_health_pct: None,
        rated_capacity_kwh: None,
        carbon_footprint_class: None,
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        cathode_material: None,
        anode_material: None,
        electrolyte_material: None,
        critical_raw_materials: None,
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        soh_methodology: None,
        operating_temp_min_c: None,
        operating_temp_max_c: None,
        rated_energy_wh: None,
        recycled_content_lead_pct: None,
        battery_weight_kg: None,
        battery_type: None,
        round_trip_efficiency_pct: None,
        internal_resistance_mohm: None,
        manufacturing_date: None,
        manufacturing_place: None,
        battery_model_id: None,
        battery_passport_number: None,
    })
}

#[test]
fn public_viewer_sees_public_fields_only() {
    let data = minimal_battery_data();
    let descriptor = battery_descriptor_with_tiers();
    let json = redact_sector_data(&data, AccessTier::Public, &descriptor);
    // Public fields must be present
    assert!(json.get("batteryChemistry").is_some());
    assert!(json.get("co2ePerUnitKg").is_some());
    assert!(json.get("nominalVoltageV").is_some());
    // Professional-gated fields must be stripped
    assert!(
        json.get("dueDiligenceUrl").is_none(),
        "due_diligence_url must be hidden from Public"
    );
    assert!(
        json.get("disassemblyInstructionsUrl").is_none(),
        "disassembly_url must be hidden from Public"
    );
}

#[test]
fn professional_viewer_sees_gated_fields() {
    let data = minimal_battery_data();
    let descriptor = battery_descriptor_with_tiers();
    let json = redact_sector_data(&data, AccessTier::Professional, &descriptor);
    assert!(
        json.get("dueDiligenceUrl").is_some(),
        "Professional must see due_diligence_url"
    );
    assert!(json.get("disassemblyInstructionsUrl").is_some());
    assert!(json.get("batteryChemistry").is_some());
}

#[test]
fn empty_access_tiers_retains_all_fields() {
    let data = minimal_battery_data();
    let descriptor = SectorDescriptor {
        key: "battery".into(),
        title: "Battery".into(),
        status: RegulatoryStatus::InForce,
        legal_basis: vec!["EU 2023/1542".into()],
        dpp_applies_from: None,
        retention_years: 10,
        schema_versions: vec!["2.0.0".into()],
        current_schema_version: "2.0.0".into(),
        product_categories: vec![],
        access_tiers: std::collections::HashMap::new(),
        plugin: None,
        notes: None,
    };
    let json = redact_sector_data(&data, AccessTier::Public, &descriptor);
    // No tiers = nothing gated = all fields visible
    assert!(json.get("dueDiligenceUrl").is_some());
    assert!(json.get("disassemblyInstructionsUrl").is_some());
    assert!(json.get("batteryChemistry").is_some());
}

// ── Helper constructors ──────────────────────────────────────────────

fn cotton_fibre(pct: f64) -> FibreEntry {
    FibreEntry {
        fibre: "cotton".into(),
        pct,
        country_of_origin: None,
    }
}

fn polyester_fibre(pct: f64) -> FibreEntry {
    FibreEntry {
        fibre: "polyester".into(),
        pct,
        country_of_origin: None,
    }
}

fn test_textile_data() -> TextileData {
    TextileData {
        fibre_composition: vec![cotton_fibre(60.0), polyester_fibre(40.0)],
        country_of_manufacturing: "BD".into(),
        care_instructions: "Machine wash 40°C".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        recycled_content_pct: None,
        carbon_footprint_kg_co2e: None,
        water_use_litres: None,
        microplastic_shedding_mg_per_wash: None,
        repair_score: None,
        durability_score: None,
        expected_wash_cycles: None,
        country_of_raw_material_origin: None,
        svhc_substances: None,
        allergens: None,
        substances_of_concern: None,
        recyclability_class: None,
        end_of_life_instructions: None,
        reuse_condition: None,
        prior_use_cycles: None,
        disassembly_instructions: None,
        spare_parts_available: None,
        product_weight_grams: None,
        repair_history_url: None,
        repair_count: None,
        pef_score: None,
    }
}

// ── Fibre composition validation ──────────────────────────────────────

#[test]
fn fibre_sum_valid_passes() {
    let fibres = vec![cotton_fibre(60.0), polyester_fibre(40.0)];
    assert!(validate_fibre_composition(&fibres).is_ok());
}

#[test]
fn fibre_sum_invalid_rejects() {
    let fibres = vec![cotton_fibre(60.0), polyester_fibre(30.0)];
    let err = validate_fibre_composition(&fibres).unwrap_err();
    assert!(err.contains("90.0"), "unexpected error: {err}");
}

#[test]
fn fibre_sum_within_tolerance_passes() {
    let fibres = vec![
        FibreEntry {
            fibre: "cotton".into(),
            pct: 98.5,
            country_of_origin: None,
        },
        FibreEntry {
            fibre: "elastane".into(),
            pct: 1.0,
            country_of_origin: None,
        },
    ];
    assert!(
        validate_fibre_composition(&fibres).is_ok(),
        "99.5% should pass ±2 tolerance"
    );
}

#[test]
fn fibre_with_valid_country_of_origin_passes() {
    let fibres = vec![
        FibreEntry {
            fibre: "cotton".into(),
            pct: 70.0,
            country_of_origin: Some("IN".into()),
        },
        FibreEntry {
            fibre: "polyester".into(),
            pct: 30.0,
            country_of_origin: Some("CN".into()),
        },
    ];
    assert!(validate_fibre_composition(&fibres).is_ok());
}

#[test]
fn fibre_with_invalid_country_of_origin_rejects() {
    let fibres = vec![FibreEntry {
        fibre: "cotton".into(),
        pct: 100.0,
        country_of_origin: Some("india".into()), // must be 2-char uppercase
    }];
    let err = validate_fibre_composition(&fibres).unwrap_err();
    assert!(
        err.contains("country_of_origin"),
        "expected country_of_origin error, got: {err}"
    );
}

// ── SVHC validation ───────────────────────────────────────────────────

#[test]
fn svhc_valid_list_passes() {
    let substances = vec![SvhcSubstance {
        cas_number: "80-05-7".into(),
        substance_name: "Bisphenol A".into(),
        concentration_pct: 0.15,
        location_in_product: Some("coating".into()),
        scip_notification_id: None,
    }];
    assert!(validate_svhc_substances(&substances).is_ok());
}

#[test]
fn svhc_empty_cas_rejects() {
    let substances = vec![SvhcSubstance {
        cas_number: "".into(),
        substance_name: "Unknown".into(),
        concentration_pct: 0.5,
        location_in_product: None,
        scip_notification_id: None,
    }];
    assert!(validate_svhc_substances(&substances).is_err());
}

#[test]
fn svhc_invalid_concentration_rejects() {
    let substances = vec![SvhcSubstance {
        cas_number: "80-05-7".into(),
        substance_name: "Bisphenol A".into(),
        concentration_pct: -1.0, // invalid
        location_in_product: None,
        scip_notification_id: None,
    }];
    assert!(validate_svhc_substances(&substances).is_err());
}

#[test]
fn svhc_empty_list_passes() {
    // Empty list means manufacturer checked and found no SVHCs — valid
    assert!(validate_svhc_substances(&[]).is_ok());
}

// ── Surfactant validation ─────────────────────────────────────────────

#[test]
fn surfactants_valid_list_passes() {
    let surfactants = vec![SurfactantEntry {
        name: "Sodium laureth sulfate".into(),
        biodegradable: true,
        concentration_band: "5-15%".into(),
        cas_number: Some("9004-82-4".into()),
    }];
    assert!(validate_surfactants(&surfactants).is_ok());
}

#[test]
fn surfactants_invalid_band_rejects() {
    let surfactants = vec![SurfactantEntry {
        name: "Mystery surfactant".into(),
        biodegradable: true,
        concentration_band: "lots".into(), // not a recognised band
        cas_number: None,
    }];
    assert!(validate_surfactants(&surfactants).is_err());
}

// ── Serde round-trips ─────────────────────────────────────────────────

#[test]
fn sector_data_battery_round_trip() {
    let data = SectorData::Battery(BatteryData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: 3000,
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: Some(12.5),
        recycled_content_nickel_pct: None,
        state_of_health_pct: None,
        rated_capacity_kwh: Some(32.0),
        carbon_footprint_class: Some(CarbonFootprintClass::B),
        due_diligence_url: None,
        cathode_material: None,
        anode_material: None,
        electrolyte_material: None,
        critical_raw_materials: None,
        disassembly_instructions_url: None,
        soh_methodology: None,
        operating_temp_min_c: None,
        operating_temp_max_c: None,
        rated_energy_wh: None,
        recycled_content_lead_pct: None,
        battery_weight_kg: None,
        battery_type: None,
        round_trip_efficiency_pct: None,
        internal_resistance_mohm: None,
        manufacturing_date: None,
        manufacturing_place: None,
        battery_model_id: None,
        battery_passport_number: None,
    });
    let json = serde_json::to_value(&data).unwrap();
    assert_eq!(json["sector"], "battery", "sector tag must be lowercase");
    assert_eq!(json["batteryChemistry"], "LFP");
    assert_eq!(json["gtin"], "09506000134352");
    let back: SectorData = serde_json::from_value(json).unwrap();
    assert_eq!(data, back);
}

#[test]
fn sector_data_textile_round_trip() {
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

    let sector = SectorData::Textile(data.clone());
    let json = serde_json::to_value(&sector).unwrap();
    assert_eq!(json["sector"], "textile", "sector tag must be lowercase");
    assert_eq!(json["countryOfManufacturing"], "BD");
    assert_eq!(json["durabilityScore"], 7.5);
    assert_eq!(json["microplasticSheddingMgPerWash"], 12.3);
    assert!(json["svhcSubstances"].is_array());
    assert_eq!(json["svhcSubstances"][0]["casNumber"], "80-05-7");
    assert_eq!(
        json["fibreComposition"][0]["countryOfOrigin"], "IN",
        "per-fibre origin must serialize"
    );

    let back: SectorData = serde_json::from_value(json).unwrap();
    assert_eq!(SectorData::Textile(data), back);
}

#[test]
fn textile_none_fields_not_serialized() {
    // Verify skip_serializing_if works — None fields should be absent from JSON
    let data = SectorData::Textile(test_textile_data());
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
    // v1.0.0 JSON (without new fields) must still deserialize into the expanded struct
    let v1_json = serde_json::json!({
        "sector": "textile",
        "fibreComposition": [{"fibre": "cotton", "pct": 100.0}],
        "countryOfManufacturing": "PT",
        "careInstructions": "Hand wash",
        "chemicalComplianceStandard": "REACH"
    });
    let parsed: SectorData = serde_json::from_value(v1_json).unwrap();
    if let SectorData::Textile(t) = parsed {
        assert_eq!(t.country_of_manufacturing, "PT");
        assert!(t.svhc_substances.is_none());
        assert!(t.durability_score.is_none());
        assert!(t.microplastic_shedding_mg_per_wash.is_none());
        assert!(t.fibre_composition[0].country_of_origin.is_none());
    } else {
        panic!("expected Textile variant");
    }
}

// ── Sector enum metadata ──────────────────────────────────────────────

#[test]
fn every_sector_declares_retention_and_catalog_key() {
    // All 12 variants: minimum_retention_years() and catalog_key() must be
    // total (every match arm exercised) and consistent with the catalog.
    let all = [
        (Sector::Battery, "battery"),
        (Sector::Textile, "textile"),
        (Sector::TextileUnsoldGoods, "textile-unsold"),
        (Sector::Steel, "steel"),
        (Sector::Electronics, "electronics"),
        (Sector::Construction, "construction"),
        (Sector::Tyre, "tyre"),
        (Sector::Toy, "toy"),
        (Sector::Aluminium, "aluminium"),
        (Sector::Furniture, "furniture"),
        (Sector::Detergent, "detergent"),
        (Sector::Other, "other"),
    ];
    for (sector, key) in all {
        assert_eq!(sector.catalog_key(), key);
        // ESPR delegated acts mandate ≥ 10 years retention across the board.
        assert!(sector.minimum_retention_years() >= 10);
    }
}

#[test]
fn sector_discriminant_matches_variant() {
    let battery = SectorData::Battery(BatteryData {
        gtin: Gtin::parse("00000000000000").unwrap(),
        battery_chemistry: BatteryChemistry::Nmc,
        nominal_voltage_v: 4.0,
        nominal_capacity_ah: 50.0,
        expected_lifetime_cycles: 1000,
        co2e_per_unit_kg: 40.0,
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: None,
        recycled_content_nickel_pct: None,
        state_of_health_pct: None,
        rated_capacity_kwh: None,
        carbon_footprint_class: None,
        due_diligence_url: None,
        cathode_material: None,
        anode_material: None,
        electrolyte_material: None,
        critical_raw_materials: None,
        disassembly_instructions_url: None,
        soh_methodology: None,
        operating_temp_min_c: None,
        operating_temp_max_c: None,
        rated_energy_wh: None,
        recycled_content_lead_pct: None,
        battery_weight_kg: None,
        battery_type: None,
        round_trip_efficiency_pct: None,
        internal_resistance_mohm: None,
        manufacturing_date: None,
        manufacturing_place: None,
        battery_model_id: None,
        battery_passport_number: None,
    });
    assert_eq!(battery.sector(), Sector::Battery);
}
