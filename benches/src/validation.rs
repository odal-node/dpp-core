use criterion::{Criterion, criterion_group, criterion_main};
use dpp_domain::domain::gtin::Gtin;
use dpp_domain::domain::sector::{
    BatteryChemistry, BatteryData, FibreEntry, SectorData, TextileData,
};
use dpp_domain::domain::validation::{validate_sector_data, validate_sector_data_batch};

fn valid_battery() -> SectorData {
    SectorData::Battery(BatteryData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 48.0,
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
    })
}

fn valid_textile() -> SectorData {
    SectorData::Textile(TextileData {
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
        country_of_manufacturing: "BD".into(),
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
    })
}

fn validation_benchmarks(c: &mut Criterion) {
    // Warm the OnceLock validators so compilation cost isn't measured.
    let _ = validate_sector_data(&valid_battery());
    let _ = validate_sector_data(&valid_textile());

    let battery = valid_battery();
    let textile = valid_textile();

    c.bench_function("validate_battery", |b| {
        b.iter(|| validate_sector_data(&battery).unwrap());
    });

    c.bench_function("validate_textile", |b| {
        b.iter(|| validate_sector_data(&textile).unwrap());
    });

    let batch: Vec<SectorData> = (0..100)
        .map(|i| {
            if i % 2 == 0 {
                valid_battery()
            } else {
                valid_textile()
            }
        })
        .collect();

    c.bench_function("validate_batch_100", |b| {
        b.iter(|| validate_sector_data_batch(&batch));
    });
}

criterion_group!(benches, validation_benchmarks);
criterion_main!(benches);
