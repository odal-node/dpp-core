//! Serde round-trip, state-machine, validation, and redaction tests for `Passport`.

use super::*;
use crate::domain::gtin::Gtin;
use crate::domain::identity::AccessTier;
use crate::domain::sector::{
    BatteryChemistry, BatteryData, CarbonFootprint, RepairabilityScore, Sector, SectorData,
};
use crate::domain::status::PassportStatus;
use chrono::Utc;
use uuid::Uuid;

fn make_passport() -> Passport {
    Passport {
        id: PassportId(uuid::Uuid::nil()),
        batch_id: Some("BATCH-001".to_owned()),
        product_name: "Eco Widget".to_owned(),
        sector: Sector::Electronics,
        product_category: Some(ProductCategory::Smartphone),
        manufacturer: ManufacturerInfo {
            name: "ACME Corp".to_owned(),
            address: "123 Main St, Berlin, DE".to_owned(),
            did_web_url: Some("https://acme.example.com/.well-known/did.json".to_owned()),
        },
        materials: vec![MaterialEntry {
            name: "Recycled Aluminium".to_owned(),
            weight_kg: 0.5,
            recycled_pct: Some(80.0),
            origin_country: Some("DE".to_owned()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(2.5)),
        repairability_score: Some(RepairabilityScore::from_scalar(7.5)),
        compliance_result: None,
        lint_result: None,
        sector_data: None,
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        published_at: None,
        schema_version: "1.0.0".to_owned(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        retention_until: None,
        product_id: None,
        operator_identifier: None,
        facility: None,
        seal: None,
    }
}

#[test]
fn passport_serde_round_trip() {
    let passport = make_passport();
    let json = serde_json::to_string(&passport).expect("serialise");
    let back: Passport = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(passport.id, back.id);
    assert_eq!(passport.product_name, back.product_name);
    assert_eq!(passport.status, back.status);
    assert_eq!(passport.schema_version, back.schema_version);
}

#[test]
fn passport_carries_typed_sector_and_category() {
    let json = serde_json::to_value(make_passport()).expect("serialise");
    assert_eq!(json["sector"], "electronics"); // Sector → camelCase
    assert_eq!(json["productCategory"], "smartphone"); // ProductCategory → snake_case
    let back: Passport = serde_json::from_value(json).expect("deserialise");
    assert_eq!(back.sector, Sector::Electronics);
    assert_eq!(back.product_category, Some(ProductCategory::Smartphone));
}

#[test]
fn sector_data_mismatch_fails_validation() {
    let mut p = make_passport(); // sector = Electronics
    p.sector_data = Some(SectorData::Battery(BatteryData {
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
    }));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("sector must match"), "got: {err}");
}

#[test]
fn passport_json_uses_camel_case() {
    let passport = make_passport();
    let json = serde_json::to_value(&passport).expect("serialise");
    assert!(
        json.get("productName").is_some(),
        "expected camelCase productName"
    );
    assert!(
        json.get("createdAt").is_some(),
        "expected camelCase createdAt"
    );
    assert!(
        json.get("schemaVersion").is_some(),
        "expected camelCase schemaVersion"
    );
}

#[test]
fn passport_status_serialises_published_as_active() {
    let json = serde_json::to_value(PassportStatus::Published).expect("serialise");
    assert_eq!(json.as_str().unwrap(), "active");
}

#[test]
fn passport_status_deserialises_both_active_and_published() {
    let from_active: PassportStatus = serde_json::from_str("\"active\"").unwrap();
    let from_published: PassportStatus = serde_json::from_str("\"published\"").unwrap();
    assert_eq!(from_active, PassportStatus::Published);
    assert_eq!(from_published, PassportStatus::Published);
}

#[test]
fn transition_draft_to_published_sets_retention_lock() {
    let mut p = make_passport();
    assert_eq!(p.status, PassportStatus::Draft);
    assert!(!p.retention_locked);
    assert!(p.published_at.is_none());

    p.transition_to(PassportStatus::Published).unwrap();

    assert_eq!(p.status, PassportStatus::Published);
    assert!(p.retention_locked);
    assert!(p.published_at.is_some());
}

#[test]
fn transition_invalid_returns_error() {
    let mut p = make_passport();
    // Draft → Suspended is not a valid transition
    let err = p.transition_to(PassportStatus::Suspended);
    assert!(err.is_err());
    // Status should remain unchanged
    assert_eq!(p.status, PassportStatus::Draft);
}

#[test]
fn transition_archived_is_terminal() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Archived).unwrap();
    assert_eq!(p.status, PassportStatus::Archived);

    // Archived → anything is invalid
    assert!(p.transition_to(PassportStatus::Published).is_err());
    assert!(p.transition_to(PassportStatus::Draft).is_err());
}

#[test]
fn transition_re_publish_does_not_overwrite_published_at() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    let first_published = p.published_at;

    p.transition_to(PassportStatus::Suspended).unwrap();
    p.transition_to(PassportStatus::Published).unwrap();

    // published_at should retain the original timestamp
    assert_eq!(p.published_at, first_published);
}

// ── validate() tests ──────────────────────────────────────────────

#[test]
fn validate_valid_passport_ok() {
    let p = make_passport();
    assert!(p.validate().is_ok());
}

#[test]
fn validate_empty_product_name() {
    let mut p = make_passport();
    p.product_name = "".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("product_name"), "got: {err}");
}

#[test]
fn validate_empty_manufacturer_name() {
    let mut p = make_passport();
    p.manufacturer.name = "   ".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("manufacturer.name"), "got: {err}");
}

#[test]
fn validate_empty_manufacturer_address() {
    let mut p = make_passport();
    p.manufacturer.address = "".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("manufacturer.address"), "got: {err}");
}

#[test]
fn validate_invalid_semver() {
    let mut p = make_passport();
    p.schema_version = "v1".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("schema_version"), "got: {err}");
}

#[test]
fn validate_negative_co2e() {
    let mut p = make_passport();
    p.co2e_per_unit = Some(CarbonFootprint::from_kg(-1.0));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("co2e_per_unit"), "got: {err}");
}

#[test]
fn validate_repairability_out_of_range() {
    let mut p = make_passport();
    p.repairability_score = Some(RepairabilityScore::from_scalar(11.0));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("repairability_score"), "got: {err}");
}

#[test]
fn validate_multiple_errors_joined() {
    let mut p = make_passport();
    p.product_name = "".to_owned();
    p.manufacturer.name = "".to_owned();
    p.co2e_per_unit = Some(CarbonFootprint::from_kg(-5.0));
    let err = p.validate().unwrap_err().to_string();
    // All three issues should appear, separated by semicolons
    assert!(err.contains("product_name"), "got: {err}");
    assert!(err.contains("manufacturer.name"), "got: {err}");
    assert!(err.contains("co2e_per_unit"), "got: {err}");
}

#[test]
fn validate_none_optionals_ok() {
    let mut p = make_passport();
    p.co2e_per_unit = None;
    p.repairability_score = None;
    assert!(p.validate().is_ok());
}

#[test]
fn v02_fields_round_trip() {
    let mut p = make_passport();
    let predecessor_id = PassportId(Uuid::now_v7());
    p.version = 2;
    p.supersedes_id = Some(predecessor_id);
    p.retention_until = Some(Utc::now() + chrono::Duration::days(3650));
    p.product_id = Some(Uuid::now_v7());
    p.operator_identifier = Some("DE12345678".to_owned());
    p.facility = Some(crate::domain::passport::FacilitySnapshot {
        scheme: "national".to_owned(),
        value: "FAC-001".to_owned(),
        name: "Plant One".to_owned(),
        country: "DE".to_owned(),
        address: None,
    });

    let json = serde_json::to_string(&p).unwrap();
    let back: Passport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, 2);
    assert_eq!(back.supersedes_id, Some(predecessor_id));
    assert!(back.retention_until.is_some());
    assert_eq!(back.operator_identifier.as_deref(), Some("DE12345678"));
    assert_eq!(
        back.facility.as_ref().map(|f| f.value.as_str()),
        Some("FAC-001")
    );
    assert_eq!(
        back.facility.as_ref().map(|f| f.name.as_str()),
        Some("Plant One")
    );
}

#[test]
fn published_to_superseded_is_valid_transition() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Superseded).unwrap();
    assert_eq!(p.status, PassportStatus::Superseded);
}

#[test]
fn superseded_is_terminal() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Superseded).unwrap();
    assert!(p.transition_to(PassportStatus::Published).is_err());
    assert!(p.transition_to(PassportStatus::Archived).is_err());
    assert!(p.transition_to(PassportStatus::Draft).is_err());
}

#[test]
fn default_version_is_one_and_skipped_when_none_optional_fields_absent() {
    let p = make_passport();
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["version"], 1);
    assert!(json.get("supersedes_id").is_none() || json["supersedes_id"].is_null());
    assert!(json.get("retentionUntil").is_none() || json["retentionUntil"].is_null());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_wires_sector_data_validation() {
    use crate::domain::sector::{FibreEntry, TextileData};
    let mut p = make_passport();
    p.sector = Sector::Textile;
    p.sector_data = Some(SectorData::Textile(TextileData {
        gtin: "09506000134352".into(),
        // fibre sum = 50% — cross-field rule must catch this
        fibre_composition: vec![FibreEntry {
            fibre: "cotton".into(),
            pct: 50.0,
            country_of_origin: None,
        }],
        country_of_manufacturing: "DE".into(),
        care_instructions: "Machine wash 30°C".into(),
        chemical_compliance_standard: "REACH".into(),
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
    }));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("fibreComposition") || err.contains("fibre"),
        "expected fibre error from sector_data validation, got: {err}"
    );
}

#[test]
fn sector_data_preserved_round_trip() {
    let mut passport = make_passport();
    passport.sector = Sector::Battery; // keep sector consistent with the data
    passport.sector_data = Some(SectorData::Battery(BatteryData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: 3000,
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: None,
        recycled_content_nickel_pct: None,
        state_of_health_pct: Some(95.3),
        rated_capacity_kwh: Some(32.0),
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
    }));
    let json = serde_json::to_string(&passport).unwrap();
    let back: Passport = serde_json::from_str(&json).unwrap();
    if let Some(SectorData::Battery(ref b)) = back.sector_data {
        assert_eq!(b.battery_chemistry, BatteryChemistry::Lfp);
        assert_eq!(b.state_of_health_pct, Some(95.3));
    } else {
        panic!("expected Battery sector data");
    }
}

// ── redact() tests ────────────────────────────────────────────────────

fn battery_passport_with_due_diligence() -> Passport {
    let mut p = make_passport();
    p.sector = Sector::Battery;
    p.batch_id = Some("BATCH-42".into());
    p.jws_signature = Some("eyJhbGci.test.signature".into());
    p.sector_data = Some(SectorData::Battery(BatteryData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: 3000,
        co2e_per_unit_kg: 85.4,
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: None,
        recycled_content_nickel_pct: None,
        state_of_health_pct: None,
        rated_capacity_kwh: None,
        carbon_footprint_class: None,
        cathode_material: None,
        anode_material: None,
        electrolyte_material: None,
        critical_raw_materials: None,
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
    }));
    p
}

#[test]
fn redact_public_strips_batch_id_jws_and_retention() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(AccessTier::Public, &catalog).into_value();
    assert!(
        view.get("batchId").is_none(),
        "batchId must be stripped at Public"
    );
    assert!(
        view.get("jwsSignature").is_none(),
        "jwsSignature must be stripped at Public"
    );
    assert!(
        view.get("retentionLocked").is_none(),
        "retentionLocked must be stripped at Public"
    );
    assert!(
        view.get("productName").is_some(),
        "productName must survive"
    );
}

#[test]
fn redact_public_strips_gated_sector_fields() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(AccessTier::Public, &catalog).into_value();
    let sd = &view["sectorData"];
    assert!(
        sd.get("dueDiligenceUrl").is_none(),
        "dueDiligenceUrl is Professional — must be hidden"
    );
    assert!(
        sd.get("disassemblyInstructionsUrl").is_none(),
        "disassemblyInstructionsUrl is Professional"
    );
    assert!(
        sd.get("batteryChemistry").is_some(),
        "batteryChemistry is Public — must survive"
    );
    assert!(
        sd.get("co2ePerUnitKg").is_some(),
        "co2ePerUnitKg is Public — must survive"
    );
}

#[test]
fn redact_professional_exposes_gated_sector_fields() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(AccessTier::Professional, &catalog).into_value();
    let sd = &view["sectorData"];
    assert!(
        sd.get("dueDiligenceUrl").is_some(),
        "Professional must see dueDiligenceUrl"
    );
    assert!(sd.get("disassemblyInstructionsUrl").is_some());
    // Still no JWS / retentionLocked at Professional
    assert!(view.get("jwsSignature").is_none());
    assert!(view.get("retentionLocked").is_none());
    // But batchId is visible
    assert!(view.get("batchId").is_some());
}

#[test]
fn redact_confidential_exposes_everything() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(AccessTier::Confidential, &catalog).into_value();
    assert!(view.get("batchId").is_some());
    assert!(view.get("jwsSignature").is_some());
    assert!(view.get("retentionLocked").is_some());
    let sd = &view["sectorData"];
    assert!(sd.get("dueDiligenceUrl").is_some());
}

#[test]
fn redact_no_sector_data_leaves_passport_fields() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = make_passport(); // no sector_data, no batchId
    let view = p.redact(AccessTier::Public, &catalog).into_value();
    assert!(view.get("productName").is_some());
    assert!(view.get("sectorData").is_none());
}
