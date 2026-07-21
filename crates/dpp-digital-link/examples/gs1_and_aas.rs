//! Example: Parse a GS1 Digital Link and map a typed Passport to a full AAS shell.
//!
//! Demonstrates the interoperability layer — GS1 URI parsing and automatic
//! conversion to IDTA Asset Administration Shell structures for Industry 4.0 /
//! Catena-X integration.
//!
//! Run with: `cargo run --example gs1_and_aas`

use chrono::Utc;
use dpp_digital_link::{
    AasSubmodelElement, AccessTier, DigitalLink, DppMediaType, Gs1LinkType, LinkDescriptor,
    ResolutionRequest, build_aas_from_passport, negotiate,
};
use dpp_domain::{
    CarbonFootprint, FibreEntry, Gtin, ManufacturerInfo, MaterialEntry, Passport, PassportId,
    PassportStatus, RepairabilityScore, Sector, SectorData, TextileData,
};

fn main() {
    println!("=== GS1 Digital Link Parsing ===\n");

    let uri = "https://id.odal-node.io/01/09506000134352/21/SN-2026-001";
    let link = DigitalLink::parse(uri).unwrap();

    println!("Parsed: {uri}");
    println!("  Resolver: {}", link.resolver_base);
    println!("  GTIN (AI 01): {}", link.gtin);
    println!(
        "  Serial (AI 21): {}",
        link.serial.as_deref().unwrap_or("—")
    );
    println!("  Batch (AI 10): {}", link.batch.as_deref().unwrap_or("—"));

    let uri_batch = "https://id.odal-node.io/01/09506000134352/10/LOT-Q2-2026/21/UNIT-042";
    let link_batch = DigitalLink::parse(uri_batch).unwrap();
    println!("\nParsed: {uri_batch}");
    println!("  GTIN: {}", link_batch.gtin);
    println!("  Batch: {}", link_batch.batch.as_deref().unwrap_or("—"));
    println!("  Serial: {}", link_batch.serial.as_deref().unwrap_or("—"));

    let built = link.build();
    println!("\nRebuilt URI: {built}");

    println!("\n=== Link-Type Negotiation ===\n");

    let descriptors = vec![
        LinkDescriptor {
            link_type: Gs1LinkType::DigitalProductPassport,
            media_type: DppMediaType::Json,
            min_access_tier: AccessTier::Public,
            href: "https://api.odal-node.io/dpp/09506000134352/data".into(),
            title: Some("DPP JSON".into()),
            language: None,
        },
        LinkDescriptor {
            link_type: Gs1LinkType::DigitalProductPassport,
            media_type: DppMediaType::JsonLd,
            min_access_tier: AccessTier::Public,
            href: "https://api.odal-node.io/dpp/09506000134352/data.jsonld".into(),
            title: Some("DPP JSON-LD".into()),
            language: None,
        },
        LinkDescriptor {
            link_type: Gs1LinkType::ProductInformationPage,
            media_type: DppMediaType::Html,
            min_access_tier: AccessTier::Public,
            href: "https://passport.odal-node.io/09506000134352".into(),
            title: Some("Human-readable passport".into()),
            language: Some("en".into()),
        },
    ];

    let request = ResolutionRequest {
        link_type: Some(Gs1LinkType::DigitalProductPassport),
        media_type: Some(DppMediaType::Json),
        access_tier: None,
    };

    let resolved = negotiate(&descriptors, &request);
    println!("Negotiation request: JSON Digital Product Passport");
    println!("  Resolved: {}", resolved.unwrap().href);

    println!("\n=== AAS Shell + Submodel Mapping ===\n");

    // Build a typed Passport for a textile product
    let passport = Passport {
        id: PassportId::new(),
        batch_id: Some("LOT-Q2-2026".into()),
        product_name: "EcoWear Organic T-Shirt".into(),
        sector: Sector::Textile,
        product_category: None,
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
                origin_country: Some("IN".into()),
            },
            MaterialEntry {
                name: "Recycled Polyester".into(),
                weight_kg: 0.07,
                recycled_pct: Some(100.0),
                origin_country: Some("DE".into()),
            },
        ],
        co2e_per_unit: Some(CarbonFootprint::from_kg(8.2)),
        repairability_score: Some(RepairabilityScore::from_scalar(7.0)),
        compliance_result: None,
        lint_result: None,
        sector_data: Some(SectorData::Textile(TextileData {
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
            country_of_manufacturing: "PT".into(),
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
        })),
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        published_at: None,
        schema_version: "1.0.0".into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        parent_passport_ref: None,
        component_refs: Vec::new(),
        retention_until: None,
        product_id: None,
        operator_identifier: None,
        facility: None,
        seal: None,
    };

    let gtin = "09506000134352";
    let (shell, submodels) = build_aas_from_passport(&passport, gtin);

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
                    let unit = p
                        .unit
                        .as_deref()
                        .map(|u| format!(" [{u}]"))
                        .unwrap_or_default();
                    println!("    Property  {} = {}{}", p.id_short, p.value, unit);
                }
                AasSubmodelElement::SubmodelElementCollection(c) => {
                    println!("    Collection {} ({} children)", c.id_short, c.value.len());
                }
                AasSubmodelElement::Reference(r) => {
                    println!("    Reference  {} -> {}", r.id_short, r.value);
                }
            }
        }
    }
}
