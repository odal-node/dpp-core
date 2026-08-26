use criterion::{Criterion, criterion_group, criterion_main};
use dpp_domain::identifier::Gtin;
use dpp_domain::product_group::{
    BatteryChemistry, BatteryData, BatteryType, FibreEntry, ProductGroupData, TextileData,
};
use dpp_domain::validation::{validate_product_group_data, validate_product_group_data_batch};

fn valid_battery() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(BatteryData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 48.0,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: Some(3000),
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: None,
        recycled_content_nickel_pct: None,
        state_of_health_pct: None,
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
        expected_lifetime: None,
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
    }))
}

fn valid_textile() -> ProductGroupData {
    ProductGroupData::Textile(Box::new(TextileData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        fibre_composition: vec![
            FibreEntry {
                fibre: "cotton".into(),
                pct: 60.0,
                country_of_origin: None,
            },
            FibreEntry {
                fibre: "polyester".into(),
                pct: 40.0,
                country_of_origin: None,
            },
        ],
        country_of_origin: "BD".into(),
        care_instructions: "30\u{00B0}C machine wash".into(),
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
    }))
}

fn validation_benchmarks(c: &mut Criterion) {
    // Warm the OnceLock validators so compilation cost isn't measured.
    let _ = validate_product_group_data(&valid_battery());
    let _ = validate_product_group_data(&valid_textile());

    let battery = valid_battery();
    let textile = valid_textile();

    c.bench_function("validate_battery", |b| {
        b.iter(|| validate_product_group_data(&battery).unwrap());
    });

    c.bench_function("validate_textile", |b| {
        b.iter(|| validate_product_group_data(&textile).unwrap());
    });

    let batch: Vec<ProductGroupData> = (0..100)
        .map(|i| {
            if i % 2 == 0 {
                valid_battery()
            } else {
                valid_textile()
            }
        })
        .collect();

    c.bench_function("validate_batch_100", |b| {
        b.iter(|| validate_product_group_data_batch(&batch));
    });
}

criterion_group!(benches, validation_benchmarks);
criterion_main!(benches);
