//! Shared fixture builders for this crate's own unit tests.
//!
//! Every one of `Passport`/`BatteryData`/`TextileData` has 20+ fields, and
//! before this module each test file hand-rolled its own full struct literal
//! — a new field silently doesn't get set at whichever call sites nobody
//! remembered to update. Build fixtures via struct-update syntax instead:
//! `Passport { product_name: "X".into(), ..sample_passport() }`.

use chrono::Utc;

use crate::domain::gtin::Gtin;
use crate::domain::passport::{ManufacturerInfo, Passport, PassportId};
use crate::domain::sector::{BatteryChemistry, BatteryData, BatteryType, Sector, TextileData};
use crate::domain::status::PassportStatus;

/// A minimal, valid `Passport` with no sector data.
pub(crate) fn sample_passport() -> Passport {
    let now = Utc::now();
    Passport {
        id: PassportId::new(),
        batch_id: None,
        product_name: "Test Product".into(),
        sector: Sector::Textile,
        manufacturer: ManufacturerInfo {
            name: "Test Manufacturer".into(),
            address: "Berlin, DE".into(),
            did_web_url: None,
        },
        materials: vec![],
        co2e_per_unit: None,
        repairability_score: None,
        compliance_result: None,
        lint_result: None,
        sector_data: None,
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        disclosure_signatures: Default::default(),
        created_at: now,
        updated_at: now,
        published_at: None,
        schema_version: "1.0.0".into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        parent_passport_ref: None,
        component_refs: Vec::new(),
        retention_until: None,
        product_id: None,
        commodity_code: None,
        operator_identifier: None,
        facility: None,
        seal: None,
    }
}

/// A minimal, valid `BatteryData` (LFP chemistry, a real check-digit-valid GTIN).
pub(crate) fn sample_battery_data() -> BatteryData {
    BatteryData {
        // 09506000134352 — verified valid GTIN-14, used throughout the test suite.
        gtin: Gtin::parse("09506000134352").expect("valid GTIN literal"),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: Some(3000),
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: None,
        recycled_content_nickel_pct: None,
        state_of_health_pct: None,
        state_of_health: None,
        expected_lifetime: None,
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
        recycled_content_reporting_year: None,
        rated_capacity_kwh: None,
        carbon_footprint_class: None,
        carbon_footprint_class_ruleset_id: None,
        carbon_footprint_class_ruleset_version: None,
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
        battery_type: BatteryType::Portable,
        initial_round_trip_efficiency_pct: None,
        round_trip_efficiency_at_half_cycle_life_pct: None,
        round_trip_efficiency_pct: None,
        internal_resistance_mohm: None,
        internal_cell_resistance_mohm: None,
        internal_pack_resistance_mohm: None,
        placed_on_market_date: None,
        manufacturing_date: None,
        manufacturing_place: None,
        battery_model_id: None,
        battery_passport_number: None,
    }
}

/// A minimal, valid `TextileData` with an empty fibre composition — tests
/// exercising fibre-sum rules override `fibre_composition` explicitly.
pub(crate) fn sample_textile_data() -> TextileData {
    TextileData {
        gtin: Gtin::parse("09506000134352").expect("valid GTIN literal"),
        fibre_composition: vec![],
        country_of_origin: "PT".into(),
        care_instructions: "Machine wash 30°C".into(),
        chemical_compliance_standard: "OEKO-TEX Standard 100".into(),
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
