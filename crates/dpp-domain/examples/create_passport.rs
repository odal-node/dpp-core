//! Example: Create a textile Digital Product Passport and validate it.
//!
//! Run with: `cargo run --example create_passport`

use chrono::Utc;
use dpp_domain::{
    CarbonFootprint, FibreEntry, ManufacturerInfo, MaterialEntry, Passport, PassportId,
    PassportStatus, RepairabilityScore, Sector, SectorData, TextileData,
};

fn main() {
    // 1. Build sector-specific data (Textile DPP)
    let textile_data = TextileData {
        fibre_composition: vec![
            FibreEntry {
                fibre: "organic cotton".into(),
                pct: 70.0,
                country_of_origin: Some("IN".into()),
            },
            FibreEntry {
                fibre: "recycled polyester".into(),
                pct: 30.0,
                country_of_origin: Some("DE".into()),
            },
        ],
        country_of_manufacturing: "PT".into(),
        care_instructions: "Machine wash 30°C, do not tumble dry".into(),
        chemical_compliance_standard: "OEKO-TEX Standard 100".into(),
        recycled_content_pct: Some(30.0),
        carbon_footprint_kg_co2e: Some(8.2),
        water_use_litres: Some(1200.0),
        microplastic_shedding_mg_per_wash: Some(12.5),
        repair_score: Some(7.0),
        durability_score: Some(8.0),
        expected_wash_cycles: Some(50),
        country_of_raw_material_origin: Some("IN".into()),
        svhc_substances: None,
        allergens: None,
        substances_of_concern: None,
        recyclability_class: None,
        end_of_life_instructions: None,
        reuse_condition: None,
        prior_use_cycles: None,
        disassembly_instructions: None,
        spare_parts_available: None,
        product_weight_grams: Some(250.0),
        repair_history_url: None,
        repair_count: None,
        pef_score: None,
    };

    // 2. Create the passport
    let now = Utc::now();
    let mut passport = Passport {
        id: PassportId::new(),
        batch_id: Some("BATCH-2026-Q2-001".into()),
        product_name: "EcoWear Organic T-Shirt".into(),
        sector: Sector::Textile,
        product_category: None,
        manufacturer: ManufacturerInfo {
            name: "GreenThread GmbH".into(),
            address: "Torstrasse 12, 10119 Berlin, DE".into(),
            did_web_url: Some("https://greenthread.example.com/.well-known/did.json".into()),
        },
        materials: vec![
            MaterialEntry {
                name: "Organic Cotton".into(),
                weight_kg: 0.175,
                recycled_pct: None,
                origin_country: Some("IN".into()),
            },
            MaterialEntry {
                name: "Recycled Polyester".into(),
                weight_kg: 0.075,
                recycled_pct: Some(100.0),
                origin_country: Some("DE".into()),
            },
        ],
        co2e_per_unit: Some(CarbonFootprint::from_kg(8.2)),
        repairability_score: Some(RepairabilityScore::from_scalar(7.0)),
        sector_data: Some(SectorData::Textile(textile_data)),
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        created_at: now,
        updated_at: now,
        published_at: None,
        schema_version: "1.1.0".into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        retention_until: None,
        product_id: None,
        operator_identifier: None,
        facility_id: None,
    };

    println!("Created passport: {}", passport.id);
    println!("Product: {}", passport.product_name);
    println!("Manufacturer: {}", passport.manufacturer.name);
    println!("Status: {}", passport.status);

    // 3. Validate sector data against the embedded JSON schema
    #[cfg(not(target_arch = "wasm32"))]
    {
        use dpp_domain::validate_sector_data;

        match validate_sector_data(passport.sector_data.as_ref().unwrap()) {
            Ok(()) => println!("✓ Sector data validates against textile v1.1.0 schema"),
            Err(errors) => {
                eprintln!("✗ Validation failed:");
                for e in &errors.errors {
                    eprintln!("  {}: {}", e.field, e.message);
                }
                std::process::exit(1);
            }
        }
    }

    // 4. Transition Draft → Published (enforces state machine)
    passport.transition_to(PassportStatus::Published).unwrap();
    println!("✓ Published at: {}", passport.published_at.unwrap());
    println!("  Retention locked: {}", passport.retention_locked);

    // 5. Serialise to JSON (what would be stored / transmitted)
    let json = serde_json::to_string_pretty(&passport).unwrap();
    println!(
        "\nPassport JSON (first 500 chars):\n{}…",
        &json[..json.len().min(500)]
    );
}
