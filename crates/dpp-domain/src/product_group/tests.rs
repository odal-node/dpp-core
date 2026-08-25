//! Redaction by audience, and the cross-field validators product-group data
//! runs through.

use super::*;
use crate::catalog::ProductGroupDescriptor;
use crate::disclosure::Audience;
use crate::disclosure::Disclosure;
use crate::validation::rules::{
    validate_fibre_composition, validate_surfactants, validate_svhc_substances,
};

// ── redact_product_group_data ────────────────────────────────────────────────

fn battery_descriptor_with_tiers() -> ProductGroupDescriptor {
    use std::collections::HashMap;
    let mut disclosure = HashMap::new();
    disclosure.insert("dueDiligenceUrl".into(), Disclosure::Restricted);
    disclosure.insert("criticalRawMaterials".into(), Disclosure::Restricted);
    disclosure.insert("disassemblyInstructionsUrl".into(), Disclosure::Restricted);
    ProductGroupDescriptor {
        key: "battery".into(),
        title: "Battery".into(),
        schema_versions: vec!["2.0.0".into()],
        current_schema_version: "2.0.0".into(),
        product_categories: vec![],
        disclosure,
        plugin: None,
        notes: None,
    }
}

pub(super) fn minimal_battery_data() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(BatteryData {
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        ..crate::test_support::sample_battery_data()
    }))
}

#[test]
fn public_viewer_sees_public_fields_only() {
    let data = minimal_battery_data();
    let descriptor = battery_descriptor_with_tiers();
    let json = redact_product_group_data(&data, Audience::Public, &descriptor);
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
    let json = redact_product_group_data(&data, Audience::LegitimateInterest, &descriptor);
    assert!(
        json.get("dueDiligenceUrl").is_some(),
        "Professional must see due_diligence_url"
    );
    assert!(json.get("disassemblyInstructionsUrl").is_some());
    assert!(json.get("batteryChemistry").is_some());
}

#[test]
fn empty_disclosure_retains_all_fields() {
    let data = minimal_battery_data();
    let descriptor = ProductGroupDescriptor {
        key: "battery".into(),
        title: "Battery".into(),
        schema_versions: vec!["2.0.0".into()],
        current_schema_version: "2.0.0".into(),
        product_categories: vec![],
        disclosure: std::collections::HashMap::new(),
        plugin: None,
        notes: None,
    };
    let json = redact_product_group_data(&data, Audience::Public, &descriptor);
    // No disclosure map = nothing gated = all fields visible
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

pub(super) fn test_textile_data() -> TextileData {
    TextileData {
        fibre_composition: vec![cotton_fibre(60.0), polyester_fibre(40.0)],
        country_of_origin: "BD".into(),
        care_instructions: "Machine wash 40°C".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        ..crate::test_support::sample_textile_data()
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
