//! Sample passports and reports the AAS tests start from.

use crate::identity::AssetIdentity;

/// The identity a test asks for: the passport's own where it has one, and a
/// caller-chosen key where it does not.
///
/// 🚨 Both arms are real. `minimal_passport` carries no product group data at
/// all, so it has no product identifier — and that is not a defective fixture,
/// it is the case the `Named` arm exists for. A helper that papered over it by
/// inventing a GTIN would be the old behaviour wearing a new type.
pub(super) fn identity_of(p: &Passport) -> AssetIdentity<'_> {
    AssetIdentity::from_passport(p).unwrap_or(AssetIdentity::Named {
        name: "assetKey",
        value: "urn:odal-node:test-asset:1",
    })
}
use chrono::Utc;
use dpp_domain::identifier::ProductIdentifier;
use dpp_domain::unsold_goods::UnsoldGoodsReport;
use dpp_domain::{
    BatteryChemistry, BatteryData, BatteryType, CarbonFootprint, CarbonFootprintClass, Gtin,
    ManufacturerInfo, MaterialComposition, MaterialEntry, Passport, PassportId, PassportStatus,
    ProductGroup, ProductGroupCatalog, RepairabilityScore,
};

/// A minimal Annex I disclosure — one line, split totalling 100.
pub(super) fn sample_unsold_goods_report() -> UnsoldGoodsReport {
    use chrono::NaiveDate;
    use dpp_domain::unsold_goods::{
        CnCategory, DiscardReason, DiscardedProductLine, DiscardedQuantity, DisclosingEntity,
        DisclosureScope, FinancialYear, LegalEntityIdentifier, WasteTreatmentSplit,
    };

    UnsoldGoodsReport {
        entity: DisclosingEntity {
            name: "Example Retail Group SA".into(),
            identifier: LegalEntityIdentifier::Euid {
                value: "LUB123456789".into(),
            },
            scope: DisclosureScope::Standalone,
        },
        financial_year: FinancialYear {
            start: NaiveDate::from_ymd_opt(2027, 1, 1).expect("valid date"),
            end: NaiveDate::from_ymd_opt(2027, 12, 31).expect("valid date"),
        },
        lines: vec![DiscardedProductLine {
            cn_categories: vec![CnCategory::parse("6203").expect("valid CN heading")],
            description: "Men's suits and trousers".into(),
            units_discarded: DiscardedQuantity::measured(1_200),
            weight_kg: DiscardedQuantity::estimated(430),
            packaging_included: false,
            reason: Some(DiscardReason::DamagedOrContaminated),
            reason_detail: None,
            treatment: WasteTreatmentSplit {
                preparing_for_reuse_pct: 20,
                recycling_pct: 50,
                other_recovery_pct: 20,
                disposal_pct: 5,
                unknown_pct: 5,
            },
        }],
        measures_taken: "Introduced pre-season demand forecasting.".into(),
        measures_planned: "Extending the donation window to twelve weeks.".into(),
    }
}

pub(super) fn minimal_passport(product_group: ProductGroup) -> Passport {
    let schema_version = ProductGroupCatalog::new()
        .get(product_group.catalog_key())
        .map_or_else(|| "1.0.0".to_owned(), |d| d.current_schema_version.clone());
    Passport {
        id: PassportId::new(),
        batch_id: Some("BATCH-001".into()),
        serial_number: None,
        product_name: "Test Product".into(),
        product_group,
        applicable_instruments: Vec::new(),
        granularity: None,
        manufacturer: ManufacturerInfo {
            name: "ACME Corp".into(),
            address: "123 Main St, Berlin, DE".into(),
            country: None,
            did_web_url: Some("https://acme.example.com/.well-known/did.json".into()),
            registered_trade_name: None,
            electronic_address: None,
        },
        materials: vec![MaterialEntry {
            name: "Aluminium".into(),
            weight_kg: 0.3,
            recycled_pct: Some(60.0),
            country_of_origin: Some("DE".into()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(12.5)),
        repairability_score: Some(RepairabilityScore::from_scalar(8.0)),
        compliance_result: None,
        lint_result: None,
        product_group_data: None,
        status: PassportStatus::Draft,
        qr_code_url: None,
        carrier_serial: None,
        jws_signature: None,
        public_jws_signature: None,
        disclosure_signatures: Default::default(),
        personal_data: Default::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        published_at: None,
        placed_on_market_date: None,
        // Derived from the product group, not hardcoded. Disclosure is sourced from the
        // declared schema version, so a fixture claiming a version that never
        // held its fields would classify none of them — which is a fixture
        // defect that used to be invisible because the catalog map ignored
        // versions entirely.
        schema_version,
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        derived_from: Vec::new(),
        component_refs: Vec::new(),
        life_status: None,
        retention_until: None,
        product_id: None,
        commodity_code: None,
        operator_identifier: None,
        responsible_operator: None,
        facility: None,
        seal: None,
    }
}

// ── Property helper tests ─────────────────────────────────────────────

pub(super) fn battery_data_with_due_diligence() -> BatteryData {
    BatteryData {
        product_identifier: ProductIdentifier::gs1(Gtin::parse("09506000134352").unwrap()),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: Some(3000),
        co2e_per_unit_kg: Some(85.4),
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: Some(12.5),
        recycled_content_nickel_pct: None,
        state_of_health_pct: Some(95.0),
        state_of_health: None,
        hazardous_substances: None,
        usable_extinguishing_agent: None,
        renewable_content_pct: None,
        minimal_voltage_v: None,
        maximum_voltage_v: None,
        voltage_temperature_range: None,
        original_power_capability_w: None,
        power_limit_min_w: None,
        power_limit_max_w: None,
        power_temperature_range: None,
        expected_lifetime_reference_test: None,
        capacity_threshold_for_exhaustion_pct: None,
        not_in_use_temperature_range: None,
        not_in_use_temperature_reference_test: None,
        commercial_warranty_period_months: None,
        cycle_life_test_c_rate: None,
        marking_information: None,
        hazard_symbol: None,
        eu_declaration_of_conformity: None,
        waste_battery_information: None,
        component_part_numbers: None,
        spare_parts_contacts: None,
        safety_measures: None,
        test_report_results: None,
        dynamic_performance: None,
        battery_status: None,
        usage_history: None,
        due_diligence_operator_scope: None,
        expected_lifetime: None,
        recycled_content_reporting_year: None,
        rated_capacity_kwh: Some(32.0),
        carbon_footprint_class: Some(CarbonFootprintClass::new("B").expect("valid label")),
        carbon_footprint_class_ruleset_id: Some("test-cfb-classes".into()),
        carbon_footprint_class_ruleset_version: Some("0.0.0-test".into()),
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        // Annex XIII point 2(a), "detailed composition, including materials
        // used in the cathode, anode and electrolyte" — restricted, and one
        // of the few point-2 fields the battery mapper actually emits, so the
        // envelope's masking can be tested against genuinely non-public content.
        cathode_material: Some(vec![MaterialComposition {
            name: "Lithium iron phosphate".into(),
            weight_pct: 32.0,
            cas_number: None,
        }]),
        anode_material: None,
        electrolyte_material: None,
        critical_raw_materials: None,
        disassembly_instructions_url: None,
        soh_methodology: None,
        operating_temp_min_c: Some(-20.0),
        operating_temp_max_c: Some(60.0),
        rated_energy_wh: None,
        recycled_content_lead_pct: None,
        battery_weight_kg: Some(8.5),
        battery_type: BatteryType::Ev,
        initial_round_trip_efficiency_pct: None,
        round_trip_efficiency_at_half_cycle_life_pct: Some(94.0),
        round_trip_efficiency_pct: None,
        internal_resistance_mohm: None,
        internal_cell_resistance_mohm: Some(3.2),
        internal_pack_resistance_mohm: None,
        placed_on_market_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 15),
        manufacturing_date: None,
        manufacturing_place: None,
        battery_model_id: None,
        battery_passport_number: None,
    }
}
