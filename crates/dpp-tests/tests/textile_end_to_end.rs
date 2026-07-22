//! End-to-end integration test: Textile DPP lifecycle.
//!
//! This test exercises the full lifecycle of a textile Digital Product Passport
//! across multiple dpp-core crates:
//!
//! 1. Create a Passport with TextileData (dpp-domain)
//! 2. Validate sector data against the v1.1.0 textile schema (dpp-domain::schemas)
//! 3. Parse a GS1 Digital Link for the product (dpp-digital-link)
//! 4. Map the passport to an AAS submodel (dpp-digital-link::aas)
//! 5. Issue a Verifiable Credential for a repairer (dpp-crypto)
//! 6. Verify the credential and apply access tier filtering (dpp-crypto)
//! 7. Serialise / deserialise the full passport round-trip

use chrono::Utc;
use dpp_crypto::access::credential::verify_credential_claims;
use dpp_crypto::access::credential::{
    AccessTier, CredentialBuilder, CredentialRole, DppCredentialSubject,
};
use dpp_crypto::access::{SectorAccessPolicy, filter_by_access_tier};
use dpp_digital_link::DigitalLink;
use dpp_digital_link::aas::map_dpp_to_aas_submodel;
use dpp_domain::{
    CarbonFootprint, FibreEntry, Gtin, ManufacturerInfo, MaterialEntry, Passport,
    RepairabilityScore, Sector, SectorData, SvhcSubstance, TextileData,
};
use dpp_tests::fixtures::base_passport;

/// Build a realistic textile passport for testing.
fn make_textile_passport() -> Passport {
    let now = Utc::now();
    Passport {
        batch_id: Some("LOT-2026-T-0451".into()),
        product_name: "EcoWeave Organic Cotton T-Shirt".into(),
        manufacturer: ManufacturerInfo {
            name: "EcoTextile GmbH".into(),
            address: "Friedrichstraße 123, 10117 Berlin, DE".into(),
            did_web_url: Some("https://ecotextile.example.com/.well-known/did.json".into()),
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
        co2e_per_unit: Some(CarbonFootprint::from_kg(8.5)),
        repairability_score: Some(RepairabilityScore::from_scalar(6.5)),
        created_at: now,
        updated_at: now,
        ..base_passport(
            Sector::Textile,
            SectorData::Textile(TextileData {
                gtin: Gtin::parse("09506000134352").expect("valid GTIN literal"),
                fibre_composition: vec![
                    FibreEntry {
                        fibre: "cotton".into(),
                        pct: 72.0,
                        country_of_origin: Some("IN".into()),
                    },
                    FibreEntry {
                        fibre: "recycled_polyester".into(),
                        pct: 28.0,
                        country_of_origin: Some("DE".into()),
                    },
                ],
                country_of_origin: "BD".into(),
                care_instructions: "Machine wash 30°C, do not tumble dry, iron low".into(),
                chemical_compliance_standard: "OEKO-TEX 100".into(),
                recycled_content_pct: Some(28.0),
                carbon_footprint_kg_co2e: Some(8.5),
                water_use_litres: Some(2700.0),
                microplastic_shedding_mg_per_wash: Some(12.5),
                repair_score: Some(6.5),
                durability_score: Some(7.5),
                expected_wash_cycles: Some(50),
                country_of_raw_material_origin: Some("IN".into()),
                svhc_substances: Some(vec![SvhcSubstance {
                    cas_number: "80-05-7".into(),
                    substance_name: "Bisphenol A".into(),
                    concentration_pct: 0.15,
                    location_in_product: Some("coating".into()),
                    scip_notification_id: Some("SCIP-2025-001234".into()),
                }]),
                allergens: None,
                substances_of_concern: None,
                recyclability_class: Some("mono-material".into()),
                end_of_life_instructions: Some("Return to store for textile recycling".into()),
                reuse_condition: None,
                prior_use_cycles: Some(0),
                disassembly_instructions: Some(
                    "Remove buttons, separate layers by colour group".into(),
                ),
                spare_parts_available: Some(true),
                product_weight_grams: Some(250.0),
                repair_history_url: None,
                repair_count: None,
                pef_score: None,
            }),
            "1.1.0",
        )
    }
}

#[test]
fn textile_passport_serialisation_round_trip() {
    let passport = make_textile_passport();
    let json = serde_json::to_value(&passport).unwrap();
    let back: Passport = serde_json::from_value(json.clone()).unwrap();

    assert_eq!(back.id, passport.id);
    assert_eq!(back.product_name, "EcoWeave Organic Cotton T-Shirt");
    assert_eq!(back.sector, Sector::Textile);

    // Sector data survived round-trip
    if let Some(SectorData::Textile(td)) = &back.sector_data {
        assert_eq!(td.fibre_composition.len(), 2);
        assert_eq!(td.country_of_origin, "BD");
        assert!(td.svhc_substances.as_ref().unwrap().len() == 1);
    } else {
        panic!("expected TextileData after round-trip");
    }
}

#[test]
fn textile_passport_to_aas_submodel() {
    let passport = make_textile_passport();
    let json = serde_json::to_value(&passport).unwrap();
    let submodel_id = format!("urn:odal-node:dpp:{}", passport.id);
    let submodel = map_dpp_to_aas_submodel(&submodel_id, &json);

    assert_eq!(submodel.id_short, "DigitalProductPassport");
    assert!(submodel.id.contains(&passport.id.to_string()));
    // Should have top-level elements for each passport field
    assert!(!submodel.submodel_elements.is_empty());
    // Verify it round-trips through JSON
    let aas_json = serde_json::to_value(&submodel).unwrap();
    assert!(aas_json["idShort"].as_str().unwrap() == "DigitalProductPassport");
}

#[test]
fn gs1_digital_link_parsing_for_textile() {
    // Simulate a GS1 Digital Link URL for a textile product
    let url = "https://id.gs1.org/01/04012345678901/21/LOT-2026-T-0451";
    let link = DigitalLink::parse(url).expect("should parse valid GS1 Digital Link");

    assert_eq!(link.gtin.as_str(), "04012345678901");
    // Serial number (AI 21) captures the lot identifier
    assert_eq!(link.serial.as_deref(), Some("LOT-2026-T-0451"));

    // Build round-trips correctly
    let rebuilt = link.build();
    assert!(rebuilt.contains("04012345678901"));
}

#[test]
fn credential_issuance_and_access_tier_filtering() {
    let passport = make_textile_passport();

    // Extract just the sector data as a flat JSON for access filtering
    let sector_json = serde_json::to_value(passport.sector_data.as_ref().unwrap()).unwrap();
    // For the access policy engine, we need the inner textile fields
    let textile_fields = match &sector_json {
        serde_json::Value::Object(map) => {
            // SectorData::Textile serialises with a tag; grab the inner object
            map.get("Textile").cloned().unwrap_or(sector_json.clone())
        }
        _ => sector_json.clone(),
    };

    let policy = SectorAccessPolicy::from_catalog(&dpp_domain::SectorCatalog::new(), "textile")
        .expect("textile in catalog");

    // ── Public tier ────────────────────────────────────────────────────────
    let public_decision = filter_by_access_tier(&textile_fields, &policy, AccessTier::Public);
    // Public tier must NOT see professional fields
    assert!(
        public_decision
            .filtered_data
            .get("svhcSubstances")
            .is_none(),
        "public tier should not see SVHC data"
    );
    assert!(
        public_decision
            .filtered_data
            .get("disassemblyInstructions")
            .is_none(),
        "public tier should not see disassembly instructions"
    );
    // Public tier MUST see basic fields
    assert!(
        public_decision
            .filtered_data
            .get("fibreComposition")
            .is_some(),
        "public tier must see fibre composition"
    );

    // ── Professional tier (via VC) ─────────────────────────────────────────
    let subject = DppCredentialSubject {
        id: "did:web:repair-shop.example.com".into(),
        name: "GreenFix Textile Repair".into(),
        role: CredentialRole::AuthorisedRepairer,
        country: "DE".into(),
        sectors: vec!["textile".into()],
        product_categories: vec![],
    };
    let credential = CredentialBuilder::new("did:web:authority.example.com".into(), subject)
        .expires_in_days(365)
        .build();

    // Verify the credential
    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(result.is_valid(), "credential should be valid");

    // The credential grants Professional tier
    if let dpp_crypto::access::credential::VerificationResult::Valid { access_tier, .. } = &result {
        assert_eq!(*access_tier, AccessTier::Professional);

        let pro_decision = filter_by_access_tier(&textile_fields, &policy, *access_tier);
        // Professional MUST see SVHC and disassembly data
        assert!(
            pro_decision.filtered_data.get("svhcSubstances").is_some(),
            "professional tier must see SVHC data"
        );
        assert!(
            pro_decision
                .filtered_data
                .get("disassemblyInstructions")
                .is_some(),
            "professional tier must see disassembly instructions"
        );
    } else {
        panic!("expected Valid result from credential verification");
    }
}

#[test]
fn expired_credential_denied_access() {
    let subject = DppCredentialSubject {
        id: "did:web:expired-shop.example.com".into(),
        name: "Expired Repair Co".into(),
        role: CredentialRole::AuthorisedRepairer,
        country: "FR".into(),
        sectors: vec!["textile".into()],
        product_categories: vec![],
    };
    let mut credential =
        CredentialBuilder::new("did:web:authority.example.com".into(), subject).build();
    // Force expiration in the past
    credential.valid_until = Utc::now() - chrono::Duration::hours(1);

    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(
        matches!(
            result,
            dpp_crypto::access::credential::VerificationResult::Expired { .. }
        ),
        "expired credential must be rejected"
    );
}

#[test]
fn wrong_sector_credential_out_of_scope() {
    let subject = DppCredentialSubject {
        id: "did:web:battery-shop.example.com".into(),
        name: "BatteryFix Ltd".into(),
        role: CredentialRole::Recycler,
        country: "NL".into(),
        sectors: vec!["battery".into()],
        product_categories: vec![],
    };
    let credential =
        CredentialBuilder::new("did:web:authority.example.com".into(), subject).build();

    // Attempt to use a battery credential for textile data
    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(
        matches!(
            result,
            dpp_crypto::access::credential::VerificationResult::OutOfScope { .. }
        ),
        "battery credential must not grant textile access"
    );
}
