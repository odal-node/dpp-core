//! Example: map a typed Passport to a full AAS shell and submodels.
//!
//! Run with: `cargo run --example passport_to_aas`

use chrono::Utc;
use dpp_aas::{AasSubmodelElement, build_aas_from_passport};
use dpp_domain::Audience;
use dpp_domain::{
    CarbonFootprint, FibreEntry, Gtin, ManufacturerInfo, MaterialEntry, Passport, PassportId,
    PassportStatus, ProductGroup, ProductGroupData, RepairabilityScore, TextileData,
};

fn main() {
    // A typed Passport for a textile product.
    let passport = Passport {
        id: PassportId::new(),
        batch_id: Some("LOT-Q2-2026".into()),
        product_name: "EcoWear Organic T-Shirt".into(),
        product_group: ProductGroup::Textile,
        applicable_instruments: vec![dpp_domain::InstrumentRef::from_catalog("espr")],
        granularity: Some(dpp_domain::Granularity::Item),
        manufacturer: ManufacturerInfo {
            name: "GreenThread GmbH".into(),
            address: "Berlin, DE".into(),
            did_web_url: Some("https://greenthread.example.com/.well-known/did.json".into()),
        },
        materials: vec![
            MaterialEntry {
                name: "Organic Cotton".into(),
                weight_kg: 0.18,
                recycled_pct: None,
                country_of_origin: Some("IN".into()),
            },
            MaterialEntry {
                name: "Recycled Polyester".into(),
                weight_kg: 0.07,
                recycled_pct: Some(100.0),
                country_of_origin: Some("DE".into()),
            },
        ],
        co2e_per_unit: Some(CarbonFootprint::from_kg(8.2)),
        repairability_score: Some(RepairabilityScore::from_scalar(7.0)),
        compliance_result: None,
        lint_result: None,
        product_group_data: Some(ProductGroupData::Textile(Box::new(TextileData {
            gtin: Gtin::parse("09506000134352").expect("valid GTIN literal"),
            fibre_composition: vec![
                FibreEntry {
                    fibre: "organic cotton".into(),
                    pct: 70.0,
                    country_of_origin: Some("IN".into()),
                },
                FibreEntry {
                    fibre: "recycled polyester".into(),
                    pct: 30.0,
                    country_of_origin: None,
                },
            ],
            country_of_origin: "PT".into(),
            care_instructions: "Machine wash 30°C, do not tumble dry".into(),
            chemical_compliance_standard: "OEKO-TEX 100".into(),
            recycled_content_pct: Some(30.0),
            carbon_footprint_kg_co2e: Some(8.2),
            water_use_litres: Some(2700.0),
            microplastic_shedding_mg_per_wash: None,
            repair_score: Some(7.0),
            durability_score: Some(8.5),
            expected_wash_cycles: Some(100),
            country_of_raw_material_origin: None,
            svhc_substances: Some(vec![]),
            allergens: None,
            substances_of_concern: None,
            recyclability_class: Some("mono-material".into()),
            end_of_life_instructions: None,
            reuse_condition: None,
            prior_use_cycles: None,
            disassembly_instructions: None,
            spare_parts_available: None,
            product_weight_grams: Some(250.0),
            repair_history_url: None,
            repair_count: None,
            pef_score: None,
        }))),
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        disclosure_signatures: Default::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        published_at: None,
        placed_on_market_date: None,
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
    };

    let gtin = "09506000134352";
    let (shell, submodels) =
        build_aas_from_passport(&passport, gtin, Audience::Public).expect("masking");

    println!("AAS Shell");
    println!("  ID:             {}", shell.id);
    println!("  idShort:        {}", shell.id_short);
    println!(
        "  globalAssetId:  {}",
        shell.asset_information.global_asset_id
    );
    println!("  Specific asset IDs:");
    for sid in &shell.asset_information.specific_asset_ids {
        println!("    {} = {}", sid.name, sid.value);
    }
    println!("  Submodel references: {}", shell.submodels.len());

    println!("\nSubmodels ({} total):", submodels.len());
    for submodel in &submodels {
        println!(
            "  [{}]  id_short: {}  elements: {}",
            submodel.id,
            submodel.id_short,
            submodel.submodel_elements.len()
        );
        for elem in &submodel.submodel_elements {
            match elem {
                AasSubmodelElement::Property(p) => {
                    println!("    Property  {} = {}", p.id_short, p.value);
                }
                AasSubmodelElement::SubmodelElementCollection(c) => {
                    println!("    Collection {} ({} children)", c.id_short, c.value.len());
                }
                AasSubmodelElement::ReferenceElement(r) => {
                    let target = r.value.keys.first().map_or("", |k| k.value.as_str());
                    println!("    Reference  {} -> {}", r.id_short, target);
                }
            }
        }
    }
}
