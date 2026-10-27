//! Example: Create a battery Digital Product Passport, validate it and publish it.
//!
//! Run with: `cargo run --example create_passport`

use chrono::{NaiveDate, Utc};
use dpp_domain::schemas::VersionedSchemaRegistry;
use dpp_domain::{
    BatteryData, CarbonFootprint, ManufacturerInfo, MaterialEntry, Passport, PassportId,
    PassportStatus, ProductGroup, ProductGroupData,
};

fn main() {
    // 1. Build product group-specific data (Battery DPP). An industrial battery
    //    of 4.8 kWh: over 2 kWh, Regulation (EU) 2023/1542 Art. 77(1) requires a
    //    passport for it, and publishing checks the content its category owes.
    //    Remove any field below and step 4 names it as missing.
    let battery_data: BatteryData = serde_json::from_value(serde_json::json!({
        "productIdentifier": { "scheme": "gs1", "gtin": "09506000134352" },
        "batteryType": "industrial",
        "batteryChemistry": "LFP",
        "batteryModelId": "EX-LFP48-100",
        "manufacturingPlace": "Berlin, DE (plant BLN-1)",
        "manufacturingDate": "2027-01-15T00:00:00Z",
        "batteryWeightKg": 48.0,
        "nominalVoltageV": 48.0,
        "minimalVoltageV": 40.0,
        "maximumVoltageV": 54.75,
        "nominalCapacityAh": 100.0,
        "ratedCapacityKwh": 4.8,
        "originalPowerCapabilityW": 4800.0,
        "powerLimitMinW": 150.0,
        "powerLimitMaxW": 6500.0,
        "internalCellResistanceMohm": 0.7,
        "internalPackResistanceMohm": 11.0,
        "expectedLifetimeCycles": 3000,
        "renewableContentPct": 20.0,
        "notInUseTemperatureRange": { "minC": -20.0, "maxC": 45.0 },
        "notInUseTemperatureReferenceTest": "IEC 62660-1:2018 storage profile",
        "hazardousSubstances": [
            { "name": "Lithium hexafluorophosphate", "casNumber": "21324-40-3", "concentrationPct": 0.8 }
        ],
        "criticalRawMaterials": [
            { "name": "Lithium", "casNumber": "7439-93-2", "weightGrams": 2200.0, "countryOfOrigin": "CL" },
            { "name": "Natural graphite", "casNumber": "7782-42-5", "weightGrams": 4000.0, "countryOfOrigin": "CN" }
        ],
        "cathodeMaterial": [
            { "name": "LiFePO4", "weightPct": 95.0, "casNumber": "15365-14-7" },
            { "name": "PVDF binder", "weightPct": 3.0, "casNumber": "24937-79-9" },
            { "name": "Conductive carbon black", "weightPct": 2.0, "casNumber": "1333-86-4" }
        ],
        "anodeMaterial": [
            { "name": "Graphite", "weightPct": 96.0, "casNumber": "7782-42-5" },
            { "name": "SBR/CMC binder", "weightPct": 4.0, "casNumber": "9003-55-8" }
        ],
        "electrolyteMaterial": [
            { "name": "Ethylene carbonate", "weightPct": 40.0, "casNumber": "96-49-1" },
            { "name": "Dimethyl carbonate", "weightPct": 47.0, "casNumber": "616-38-6" },
            { "name": "Lithium hexafluorophosphate", "weightPct": 13.0, "casNumber": "21324-40-3" }
        ],
        "usableExtinguishingAgent": "Water mist or ABC dry powder; do not use CO2",
        "componentPartNumbers": ["EX-CELL-32700-LFP", "EX-BMS-16S-V3", "EX-PACK-ENCL-100"],
        "sparePartsContacts": "spares@batteries.example.com",
        "disassemblyInstructionsUrl": "https://batteries.example.com/service/lfp48/disassembly",
        "safetyMeasures": "De-energise and wait 15 min before opening. Class 0 gloves. No conductive tools.",
        "testReportResults": "IEC 62619:2022 pass; UN 38.3 pass; report EX-TR-2027-0001",
        "markingInformation": "Separate-collection symbol (crossed-out wheeled bin) per Art. 13(4); capacity 100 Ah / 4.8 kWh",
        "euDeclarationOfConformity": "EU declaration of conformity EX-2027-IND-0001, per Regulation (EU) 2023/1542 Art. 18",
        "wasteBatteryInformation": "Return to an authorised collection point or installer. Do not shred. Contains lithium; fire risk if damaged.",
        "batteryStatus": "original"
    }))
    .expect("the data matches BatteryData");

    // 2. Create the passport
    let now = Utc::now();
    let (schema_version, _) = VersionedSchemaRegistry::new()
        .latest("battery")
        .map(|(v, s)| (v.to_string(), s))
        .expect("battery schema is embedded");
    let mut passport = Passport {
        id: PassportId::new(),
        batch_id: Some("BATCH-2027-Q1-001".into()),
        serial_number: None,
        product_name: "Example LFP-48 storage battery".into(),
        product_group: ProductGroup::Battery,
        // Recorded once, from the catalog, at the moment of issuance — never
        // recomputed. See `InstrumentRef`.
        applicable_instruments: dpp_domain::InstrumentCatalog::new().instrument_refs_for("battery"),
        // No adopted act fixes a level for batteries yet, so it is left unset.
        granularity: None,
        manufacturer: ManufacturerInfo {
            name: "Example Batteries GmbH".into(),
            address: "Musterstrasse 1, 10115 Berlin, DE".into(),
            // ISO 3166-1 alpha-2, and its own field: which country an operator
            // is established in decides who carries the passport obligation, so
            // it cannot live inside the address string.
            country: Some("DE".into()),
            did_web_url: Some("https://batteries.example.com/.well-known/did.json".into()),
            registered_trade_name: None,
            electronic_address: None,
        },
        materials: vec![MaterialEntry {
            name: "Lithium iron phosphate".into(),
            weight_kg: 12.0,
            recycled_pct: None,
            country_of_origin: None,
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(45.2)),
        repairability_score: None,
        compliance_result: None,
        lint_result: None,
        product_group_data: Some(ProductGroupData::Battery(Box::new(battery_data))),
        status: PassportStatus::Draft,
        qr_code_url: None,
        carrier_serial: None,
        jws_signature: None,
        public_jws_signature: None,
        disclosure_signatures: Default::default(),
        personal_data: Default::default(),
        created_at: now,
        updated_at: now,
        published_at: None,
        placed_on_market_date: NaiveDate::from_ymd_opt(2027, 3, 1),
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
        // Annex III(k). `None` here only because this example does not model a
        // supply chain; a real passport states who is answerable and why.
        responsible_operator: None,
        facility: None,
        seal: None,
    };

    println!("Created passport: {}", passport.id);
    println!("Product: {}", passport.product_name);
    println!("Manufacturer: {}", passport.manufacturer.name);
    println!("Status: {}", passport.status);

    // 3. Validate product group data against the embedded JSON schema
    #[cfg(not(target_arch = "wasm32"))]
    {
        use dpp_domain::validate_product_group_data;

        match validate_product_group_data(passport.product_group_data.as_ref().unwrap()) {
            Ok(()) => println!(
                "✓ ProductGroup data validates against battery v{} schema",
                passport.schema_version
            ),
            Err(errors) => {
                eprintln!("✗ Validation failed:");
                for e in &errors.errors {
                    eprintln!("  {}: {}", e.field, e.message);
                }
                std::process::exit(1);
            }
        }
    }

    // 4. Transition Draft → Published (enforces the state machine, and the
    //    mandatory content the battery's category owes)
    if let Err(e) = passport.transition_to(PassportStatus::Published) {
        eprintln!("✗ Publish refused: {e}");
        std::process::exit(1);
    }
    println!("✓ Published at: {}", passport.published_at.unwrap());
    println!("  Retention locked: {}", passport.retention_locked);

    // 5. Serialise to JSON (what would be stored / transmitted)
    let json = serde_json::to_string_pretty(&passport).unwrap();
    println!(
        "\nPassport JSON (first 500 chars):\n{}…",
        &json[..json.len().min(500)]
    );
}
