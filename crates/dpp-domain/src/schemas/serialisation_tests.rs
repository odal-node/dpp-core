//! Every field a Rust type can emit is a field the current schema admits.

use super::*;

/// Every field the Rust type can emit is a field the current schema admits.
///
/// The current battery schema sets `additionalProperties: false`, so a struct
/// field with no schema property fails validation the moment it is populated.
/// That drift is invisible to any fixture built with `..base`: serde skips
/// `None`, so an unpopulated field never reaches the schema. It went unseen
/// from v2.0.0 to v2.6.0 — `manufacturingDate`, `manufacturingPlace`,
/// `batteryModelId` and `batteryPassportNumber` were emittable the whole time
/// and no schema version declared any of them, so a battery passport recording
/// when or where it was made could not be validated at all.
///
/// **The literal below is deliberately exhaustive — do not add `..base` to
/// it.** That is the entire mechanism: Rust requires every field in a struct
/// literal without a base, so adding a field to `BatteryData` stops this file
/// compiling until someone populates it here, and the assertion then forces
/// the matching schema property to exist. A base expression silently restores
/// the hole. There is no reflection available to do this at runtime; the
/// compiler is the only thing that can enforce completeness.
///
/// Resolves the version from the catalog rather than naming one, so a schema
/// bump does not need this test edited — and cannot quietly leave it asserting
/// against a superseded version.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_fully_populated_battery_serialises_into_the_current_schema() {
    use crate::identifier::Gtin;
    use crate::product_group::{
        BatteryChemistry, BatteryData, BatteryStatus, BatteryType, CarbonFootprintClass,
        CriticalRawMaterial, DynamicPerformance, EnvironmentalReading, ExpectedLifetime,
        HarmfulEvents, HazardSymbol, HazardousSubstance, MaterialComposition, StateOfChargeReading,
        StateOfHealth, TemperatureRange, UsageHistory,
    };
    use chrono::{NaiveDate, TimeZone as _, Utc};

    let range = TemperatureRange {
        min_c: -20.0,
        max_c: 60.0,
    };
    let moment = Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap();
    let day = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

    let data = BatteryData {
        gtin: Gtin::parse("09506000134352").expect("valid GTIN literal"),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: Some(3_000),
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: Some(16.0),
        recycled_content_lithium_pct: Some(6.0),
        recycled_content_nickel_pct: Some(6.0),
        state_of_health_pct: Some(97.0),
        rated_capacity_kwh: Some(64.0),
        carbon_footprint_class: Some(CarbonFootprintClass::new("B").expect("valid label")),
        carbon_footprint_class_ruleset_id: Some("eu-battery-cfb".into()),
        carbon_footprint_class_ruleset_version: Some("2026.1".into()),
        due_diligence_url: Some("https://example.invalid/due-diligence".into()),
        cathode_material: Some(vec![MaterialComposition {
            name: "Lithium iron phosphate".into(),
            weight_pct: 32.0,
            cas_number: Some("15365-14-7".into()),
        }]),
        anode_material: Some(vec![MaterialComposition {
            name: "Graphite".into(),
            weight_pct: 18.0,
            cas_number: Some("7782-42-5".into()),
        }]),
        electrolyte_material: Some(vec![MaterialComposition {
            name: "Lithium hexafluorophosphate".into(),
            weight_pct: 11.0,
            cas_number: Some("21324-40-3".into()),
        }]),
        critical_raw_materials: Some(vec![CriticalRawMaterial {
            name: "Natural graphite".into(),
            cas_number: Some("7782-42-5".into()),
            weight_grams: Some(8_500.0),
            country_of_origin: Some("MZ".into()),
        }]),
        disassembly_instructions_url: Some("https://example.invalid/disassembly".into()),
        soh_methodology: Some("IEC 62660-1:2018".into()),
        operating_temp_min_c: Some(-20.0),
        operating_temp_max_c: Some(60.0),
        rated_energy_wh: Some(64_000.0),
        recycled_content_lead_pct: Some(0.0),
        battery_weight_kg: Some(384.0),
        battery_type: BatteryType::Ev,
        initial_round_trip_efficiency_pct: Some(96.0),
        round_trip_efficiency_at_half_cycle_life_pct: Some(91.0),
        round_trip_efficiency_pct: Some(96.0),
        internal_resistance_mohm: Some(1.4),
        internal_cell_resistance_mohm: Some(1.4),
        internal_pack_resistance_mohm: Some(38.0),
        placed_on_market_date: Some(day),
        manufacturing_date: Some(moment),
        manufacturing_place: Some("PL:Wrocław".into()),
        battery_model_id: Some("LFP-64-A".into()),
        battery_passport_number: Some("URN:UUID:6F1C9D2E-0000-4000-8000-000000000000".into()),
        expected_lifetime: Some(Box::new(ExpectedLifetime {
            put_into_service_date: Some(day),
            energy_throughput_kwh: 128_000.0,
            capacity_throughput_ah: 400_000.0,
            harmful_events: HarmfulEvents {
                deep_discharge_events: Some(2),
                hours_in_extreme_temperature: Some(14.5),
                hours_charging_in_extreme_temperature: Some(1.5),
            },
            full_equivalent_cycles: 2_000.0,
        })),
        recycled_content_reporting_year: Some(2026),
        state_of_health: Some(Box::new(StateOfHealth::ElectricVehicle { soce_pct: 97.0 })),
        hazardous_substances: Some(vec![HazardousSubstance {
            name: "Nickel sulfate".into(),
            cas_number: Some("7786-81-4".into()),
            concentration_pct: Some(0.4),
        }]),
        usable_extinguishing_agent: Some("Class D dry powder".into()),
        renewable_content_pct: Some(12.5),
        minimal_voltage_v: Some(2.5),
        maximum_voltage_v: Some(4.2),
        voltage_temperature_range: Some(range),
        original_power_capability_w: Some(150_000.0),
        power_limit_min_w: Some(1_000.0),
        power_limit_max_w: Some(180_000.0),
        power_temperature_range: Some(range),
        expected_lifetime_reference_test: Some("IEC 62660-1:2018".into()),
        capacity_threshold_for_exhaustion_pct: Some(80.0),
        not_in_use_temperature_range: Some(range),
        not_in_use_temperature_reference_test: Some("IEC 62660-1:2018 clause 7".into()),
        commercial_warranty_period_months: Some(96),
        cycle_life_test_c_rate: Some(1.0),
        marking_information: Some("Separate collection symbol applied".into()),
        hazard_symbol: Some(HazardSymbol::Cadmium),
        eu_declaration_of_conformity: Some("DoC-2027-0001".into()),
        waste_battery_information: Some("https://example.invalid/waste".into()),
        component_part_numbers: Some(vec!["MOD-A1".into(), "BMS-C7".into()]),
        spare_parts_contacts: Some("spares@example.invalid".into()),
        safety_measures: Some("Isolate at the service disconnect before handling.".into()),
        test_report_results: Some("https://example.invalid/test-report".into()),
        dynamic_performance: Some(Box::new(DynamicPerformance {
            rated_capacity_ah: Some(98.0),
            capacity_fade_pct: Some(2.0),
            power_w: Some(148_000.0),
            power_fade_pct: Some(1.3),
            internal_resistance_mohm: Some(1.5),
            internal_resistance_increase_pct: Some(7.1),
            round_trip_efficiency_pct: Some(94.0),
            round_trip_efficiency_fade_pct: Some(2.1),
            expected_lifetime_cycles: Some(2_900),
            expected_lifetime_years: Some(11.5),
        })),
        battery_status: Some(BatteryStatus::Original),
        usage_history: Some(Box::new(UsageHistory {
            charge_discharge_cycles: Some(412),
            negative_events: Some(vec!["over-temperature 2026-02-11".into()]),
            operating_conditions: Some(vec![EnvironmentalReading {
                recorded_at: moment,
                temperature_c: Some(31.5),
                note: Some("fast charge".into()),
            }]),
            state_of_charge: Some(vec![StateOfChargeReading {
                recorded_at: moment,
                state_of_charge_pct: 78.0,
            }]),
        })),
    };

    let registry = VersionedSchemaRegistry::new();
    let (version, _) = registry.latest("battery").expect("battery schema exists");
    let json = serde_json::to_value(&data).expect("serialises");
    registry
        .validate("battery", version, &json)
        .expect("every emitted field must be admitted by the current schema");
}

/// The same completeness guard as the battery one above, for textile.
///
/// Battery is not special — it was only the product group where the drift happened to
/// exist. Any product group whose schema sets `additionalProperties: false` can grow a
/// struct field with no schema property and reject its own data at validation
/// time, and textile is the other product group with a real model rather than a
/// stub. Its literal is exhaustive for the same reason: **do not add `..base`.**
///
/// The eight remaining product groups are stubs of seven to ten fields; when one grows
/// a real model it should gain a fixture like this rather than rely on the
/// mechanical parity sweep that found the battery gap.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_fully_populated_textile_serialises_into_the_current_schema() {
    use crate::identifier::Gtin;
    use crate::product_group::{FibreEntry, SvhcSubstance, TextileData};

    let data = TextileData {
        gtin: Gtin::parse("09506000134352").expect("valid GTIN literal"),
        fibre_composition: vec![
            FibreEntry {
                fibre: "cotton".into(),
                pct: 70.0,
                country_of_origin: Some("PT".into()),
            },
            FibreEntry {
                fibre: "polyester".into(),
                pct: 30.0,
                country_of_origin: Some("TR".into()),
            },
        ],
        country_of_origin: "PT".into(),
        care_instructions: "Machine wash 30°C".into(),
        chemical_compliance_standard: "OEKO-TEX Standard 100".into(),
        recycled_content_pct: Some(24.0),
        carbon_footprint_kg_co2e: Some(8.5),
        water_use_litres: Some(2_400.0),
        microplastic_shedding_mg_per_wash: Some(1.7),
        repair_score: Some(6.5),
        durability_score: Some(7.5),
        expected_wash_cycles: Some(50),
        country_of_raw_material_origin: Some("IN".into()),
        svhc_substances: Some(vec![SvhcSubstance {
            cas_number: "80-05-7".into(),
            substance_name: "Bisphenol A".into(),
            concentration_pct: 0.15,
            location_in_product: Some("coating".into()),
            scip_notification_id: Some("SCIP-0001".into()),
        }]),
        allergens: Some(vec!["disperse blue 1".into()]),
        substances_of_concern: Some(vec!["PFAS".into()]),
        recyclability_class: Some("B".into()),
        end_of_life_instructions: Some("Separate trims before fibre recovery".into()),
        reuse_condition: Some("good".into()),
        prior_use_cycles: Some(1),
        disassembly_instructions: Some("Remove buttons, separate layers by colour".into()),
        spare_parts_available: Some(true),
        product_weight_grams: Some(420.0),
        repair_history_url: Some("https://example.invalid/repairs".into()),
        repair_count: Some(2),
        pef_score: Some(0.42),
    };

    let registry = VersionedSchemaRegistry::new();
    let (version, _) = registry.latest("textile").expect("textile schema exists");
    let json = serde_json::to_value(&data).expect("serialises");
    registry
        .validate("textile", version, &json)
        .expect("every emitted field must be admitted by the current schema");
}
