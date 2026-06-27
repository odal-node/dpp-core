//! Benchmark the AAS (Asset Administration Shell) mapping path —
//! `build_aas_from_passport` — for registry / data-space interoperability.
//! This runs on every passport export to a Catena-X / IDTA consumer.

use chrono::Utc;
use criterion::{Criterion, criterion_group, criterion_main};
use dpp_digital_link::aas::build_aas_from_passport;
use dpp_domain::domain::gtin::Gtin;
use dpp_domain::domain::sector::{BatteryChemistry, BatteryData, SectorData};
use dpp_domain::{
    CarbonFootprint, ManufacturerInfo, MaterialEntry, Passport, PassportId, PassportStatus, Sector,
};

const GTIN: &str = "09506000134352";

fn battery_passport() -> Passport {
    let now = Utc::now();
    Passport {
        id: PassportId::new(),
        batch_id: Some("LOT-BENCH-1".into()),
        product_name: "Bench Battery".into(),
        sector: Sector::Battery,
        product_category: None,
        manufacturer: ManufacturerInfo {
            name: "Bench Manufacturing".into(),
            address: "1 Bench Way, Berlin, DE".into(),
            did_web_url: Some("https://bench.example.com/.well-known/did.json".into()),
        },
        materials: vec![MaterialEntry {
            name: "LFP".into(),
            weight_kg: 12.0,
            recycled_pct: Some(15.0),
            origin_country: Some("CN".into()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(73.0)),
        repairability_score: None,
        compliance_result: None,
        sector_data: Some(SectorData::Battery(BatteryData {
            gtin: Gtin::parse(GTIN).unwrap(),
            battery_chemistry: BatteryChemistry::Lfp,
            nominal_voltage_v: 3.2,
            nominal_capacity_ah: 100.0,
            expected_lifetime_cycles: 6000,
            co2e_per_unit_kg: 73.0,
            recycled_content_cobalt_pct: Some(12.0),
            recycled_content_lithium_pct: Some(6.0),
            recycled_content_nickel_pct: Some(9.0),
            state_of_health_pct: Some(100.0),
            rated_capacity_kwh: Some(0.32),
            carbon_footprint_class: None,
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
            battery_type: None,
            round_trip_efficiency_pct: Some(94.5),
            internal_resistance_mohm: Some(0.8),
            manufacturing_date: Some(now),
            manufacturing_place: Some("DE".into()),
            battery_model_id: Some("BM-1".into()),
            battery_passport_number: None,
        })),
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        created_at: now,
        updated_at: now,
        published_at: None,
        schema_version: "2.0.0".into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        retention_until: None,
        product_id: None,
        operator_identifier: None,
        facility_id: None,
    }
}

fn aas_benchmarks(c: &mut Criterion) {
    let passport = battery_passport();

    c.bench_function("aas_build_from_battery_passport", |b| {
        b.iter(|| build_aas_from_passport(&passport, GTIN));
    });

    c.bench_function("aas_build_and_serialise", |b| {
        b.iter(|| {
            let (shell, submodels) = build_aas_from_passport(&passport, GTIN);
            (
                serde_json::to_string(&shell).unwrap(),
                serde_json::to_string(&submodels).unwrap(),
            )
        });
    });
}

criterion_group!(benches, aas_benchmarks);
criterion_main!(benches);
