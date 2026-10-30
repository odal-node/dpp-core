//! Fixtures shared by the bench targets.
//!
//! Each target is its own binary, so this file is pulled in with
//! `#[path = "fixtures.rs"] mod fixtures;` rather than as a library. The
//! `allow` below is for that: every target uses a different subset, and the
//! rest would otherwise fail `-D warnings` in whichever binary does not call it.
#![allow(dead_code)]

use chrono::Utc;
use dpp_crypto::keystore::KeyStore;
use dpp_domain::identifier::{Gtin, ProductIdentifier};
use dpp_domain::product_group::{BatteryChemistry, BatteryData, BatteryType, ProductGroupData};
use dpp_domain::{CarbonFootprint, ManufacturerInfo, MaterialEntry, Passport, ProductGroup};
use dpp_tests::fixtures::base_passport;

pub const GTIN: &str = "09506000134352";

/// An open key store with one generated key, in a file of its own.
pub fn keystore(key_id: &str) -> KeyStore {
    let path = std::env::temp_dir().join(format!("bench-keystore-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "bench-pass").expect("open store");
    store.generate_key(key_id).expect("generate key");
    store
}

/// A battery passport with the Annex XIII fields the benches care about filled
/// in, so masking, projection and serialisation all have real work to do.
pub fn battery_passport() -> Passport {
    let now = Utc::now();
    Passport {
        batch_id: Some("LOT-BENCH-1".into()),
        serial_number: None,
        product_name: "Bench Battery".into(),
        manufacturer: ManufacturerInfo {
            name: "Bench Manufacturing".into(),
            address: "1 Bench Way, Berlin, DE".into(),
            country: None,
            did_web_url: Some("https://bench.example.com/.well-known/did.json".into()),
            registered_trade_name: None,
            electronic_address: None,
        },
        materials: vec![MaterialEntry {
            name: "LFP".into(),
            weight_kg: 12.0,
            recycled_pct: Some(15.0),
            country_of_origin: Some("CN".into()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(73.0)),
        repairability_score: None,
        created_at: now,
        updated_at: now,
        ..base_passport(
            ProductGroup::Battery,
            ProductGroupData::Battery(Box::new(BatteryData {
                product_identifier: ProductIdentifier::gs1(Gtin::parse(GTIN).unwrap()),
                battery_chemistry: BatteryChemistry::Lfp,
                nominal_voltage_v: 3.2,
                nominal_capacity_ah: 100.0,
                expected_lifetime_cycles: Some(6000),
                co2e_per_unit_kg: Some(73.0),
                recycled_content_cobalt_pct: Some(12.0),
                recycled_content_lithium_pct: Some(6.0),
                recycled_content_nickel_pct: Some(9.0),
                state_of_health_pct: Some(100.0),
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
                rated_capacity_kwh: Some(0.32),
                carbon_footprint_class: None,
                carbon_footprint_class_ruleset_id: None,
                carbon_footprint_class_ruleset_version: None,
                due_diligence_url: Some("https://bench.example.com/dd".into()),
                cathode_material: None,
                anode_material: None,
                electrolyte_material: None,
                critical_raw_materials: None,
                disassembly_instructions_url: Some("https://bench.example.com/dis".into()),
                soh_methodology: Some("IEC 62660-1:2018".into()),
                operating_temp_min_c: Some(-20.0),
                operating_temp_max_c: Some(60.0),
                rated_energy_wh: Some(320.0),
                recycled_content_lead_pct: None,
                battery_weight_kg: Some(15.5),
                battery_type: BatteryType::Ev,
                initial_round_trip_efficiency_pct: None,
                round_trip_efficiency_at_half_cycle_life_pct: Some(94.5),
                round_trip_efficiency_pct: None,
                internal_resistance_mohm: None,
                internal_cell_resistance_mohm: Some(0.8),
                internal_pack_resistance_mohm: None,
                placed_on_market_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 15),
                manufacturing_date: Some(now),
                manufacturing_place: Some("DE".into()),
                battery_model_id: Some("BM-1".into()),
                battery_passport_number: None,
            })),
            "2.0.0",
        )
    }
}

/// The battery passport with `materials` bill-of-materials lines, for sweeps
/// over how a per-passport cost grows with the size of the record.
pub fn battery_passport_with_materials(materials: usize) -> Passport {
    Passport {
        materials: (0..materials)
            .map(|i| MaterialEntry {
                name: format!("material-{i:04}"),
                weight_kg: 0.1 + (i as f64) * 0.01,
                recycled_pct: Some((i % 100) as f64),
                country_of_origin: Some("DE".into()),
            })
            .collect(),
        ..battery_passport()
    }
}
