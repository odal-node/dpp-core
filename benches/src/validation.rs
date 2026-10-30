use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dpp_domain::ProductGroupCatalog;
use dpp_domain::identifier::{Gtin, ProductIdentifier};
use dpp_domain::product_group::{
    BatteryChemistry, BatteryData, BatteryType, FibreEntry, ProductGroupData, TextileData,
};
use dpp_domain::validation::{
    ProductGroupValidatorRegistry, validate_passport, validate_product_group_data,
    validate_product_group_data_batch, validate_raw_product_group_data,
};

#[path = "fixtures.rs"]
mod fixtures;

fn valid_battery() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(BatteryData {
        product_identifier: ProductIdentifier::gs1(Gtin::parse("09506000134352").unwrap()),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 48.0,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: Some(3000),
        co2e_per_unit_kg: Some(85.4),
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
        due_diligence_operator_scope: None,
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
        product_identifier: ProductIdentifier::gs1(Gtin::parse("09506000134352").unwrap()),
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

    // The batch above, as a ladder. The 100 point repeats the one above by design,
    // so the three sit on one curve.
    let mut ladder = c.benchmark_group("validate_batch_by_size");
    for size in [10_usize, 100, 1_000] {
        let items: Vec<ProductGroupData> = (0..size)
            .map(|i| {
                if i % 2 == 0 {
                    valid_battery()
                } else {
                    valid_textile()
                }
            })
            .collect();
        ladder.throughput(Throughput::Elements(size as u64));
        ladder.bench_with_input(BenchmarkId::from_parameter(size), &items, |b, items| {
            b.iter(|| validate_product_group_data_batch(items));
        });
    }
    ladder.finish();

    // The publish gate: the envelope, the product group data and the cross-field
    // rules together, on a whole passport rather than its payload alone.
    //
    // The shared fixture declares cobalt and nickel recycled content on an LFP
    // battery, which the cross-field rules refuse, so those two are cleared.
    let mut passport = fixtures::battery_passport();
    if let Some(ProductGroupData::Battery(data)) = &mut passport.product_group_data {
        data.recycled_content_cobalt_pct = None;
        data.recycled_content_nickel_pct = None;
    }
    validate_passport(&passport).unwrap_or_else(|e| {
        panic!("the fixture passport should pass the gate, or this times the failure path: {e:?}")
    });
    c.bench_function("validate_passport_battery", |b| {
        b.iter(|| validate_passport(&passport).unwrap());
    });
}

/// A frozen, schema-valid document for `product_group` at `version`. These are
/// the compatibility fixtures kept beside `dpp-domain`'s tests; each holds the
/// required fields only.
fn frozen_fixture(product_group: &str, version: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../crates/dpp-domain/tests/fixtures/schema-compat")
        .join(product_group)
        .join(format!("v{version}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no frozen fixture at {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("a frozen fixture is valid JSON")
}

/// Schema validation for every product group the catalog knows, at the version
/// it marks current. The typed benches above are realistic documents for two
/// groups; this is the schema cost across all of them. Because each fixture
/// holds only the required fields, the figures are a floor for that group: a
/// fully populated passport validates more slowly.
fn validation_by_product_group(c: &mut Criterion) {
    let catalog = ProductGroupCatalog::new();
    let registry = ProductGroupValidatorRegistry::default();

    let mut group = c.benchmark_group("validate_raw_by_product_group");
    for key in catalog.keys() {
        let version = catalog
            .current_schema_version(key)
            .expect("a catalog key has a current version");
        let document = frozen_fixture(key, version);

        // Compile the schema outside the measurement, and prove the fixture is
        // valid so the timed path is the success path.
        validate_raw_product_group_data(key, &document, &registry)
            .unwrap_or_else(|e| panic!("{key} v{version} fixture should validate: {e:?}"));
        // These figures are small enough to be a no-op. Show the validator is live:
        // a document that is not the group's, and says so, is refused.
        assert!(
            validate_raw_product_group_data(
                key,
                &serde_json::json!({"benchStrayProperty": true}),
                &registry
            )
            .is_err(),
            "{key} should refuse a document that is not its own"
        );

        group.bench_with_input(
            BenchmarkId::from_parameter(key),
            &document,
            |b, document| {
                b.iter(|| validate_raw_product_group_data(key, document, &registry).unwrap());
            },
        );
    }
    group.finish();
}

criterion_group!(benches, validation_benchmarks, validation_by_product_group);
criterion_main!(benches);
