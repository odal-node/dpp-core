//! End-to-end integration test: Battery DPP lifecycle.
//!
//! The battery passport is the highest-stakes sector — mandatory from
//! 18 Feb 2027 under the EU Battery Regulation (2023/1542), Annex XIII. This
//! test mirrors `textile_end_to_end.rs` for battery and exercises the full
//! lifecycle across multiple dpp-core crates:
//!
//! 1. Create a Passport with fully-populated BatteryData (dpp-domain)
//! 2. Serialise / deserialise the full passport round-trip
//! 3. Map the passport to the AAS shell + submodels (dpp-digital-link::aas)
//! 4. Parse a GS1 Digital Link for the battery (dpp-digital-link)
//! 5. Issue a Verifiable Credential for a recycler (dpp-crypto)
//! 6. Verify the credential and apply access-tier filtering (dpp-crypto)
//! 7. Redact sector data through the catalog descriptor (dpp-domain)

use chrono::Utc;
use dpp_crypto::access::credential::{
    AccessTier, CredentialBuilder, CredentialRole, DppCredentialSubject, VerificationResult,
    verify_credential_claims,
};
use dpp_crypto::access::{SectorAccessPolicy, filter_by_access_tier};
use dpp_digital_link::DigitalLink;
use dpp_digital_link::aas::build_aas_from_passport;
use dpp_domain::domain::sector::CriticalRawMaterial;
use dpp_domain::{
    BatteryChemistry, BatteryData, BatteryType, CarbonFootprint, CarbonFootprintClass, Gtin,
    ManufacturerInfo, MaterialComposition, MaterialEntry, Passport, PassportId, PassportStatus,
    RepairabilityScore, Sector, SectorCatalog, SectorData, redact_sector_data,
};

/// The canonical valid GTIN-14 used throughout the test suite.
const VALID_GTIN: &str = "09506000134352";

/// Build a realistic, fully-populated battery passport. Every `Option` field is
/// `Some` so the AAS mapper and redaction paths exercise all of their branches.
fn make_battery_passport() -> Passport {
    let now = Utc::now();
    Passport {
        id: PassportId::new(),
        batch_id: Some("LOT-2027-B-0917".into()),
        product_name: "PowerCell EV Module 4680".into(),
        sector: Sector::Battery,
        product_category: None,
        manufacturer: ManufacturerInfo {
            name: "Volt Dynamics GmbH".into(),
            address: "Industriestraße 7, 80807 München, DE".into(),
            did_web_url: Some("https://voltdynamics.example.com/.well-known/did.json".into()),
        },
        materials: vec![
            MaterialEntry {
                name: "Lithium Iron Phosphate".into(),
                weight_kg: 12.4,
                recycled_pct: Some(15.0),
                origin_country: Some("CN".into()),
            },
            MaterialEntry {
                name: "Graphite".into(),
                weight_kg: 3.1,
                recycled_pct: Some(8.0),
                origin_country: Some("MZ".into()),
            },
        ],
        co2e_per_unit: Some(CarbonFootprint::from_kg(73.2)),
        repairability_score: Some(RepairabilityScore::from_scalar(4.5)),
        compliance_result: None,
        sector_data: Some(SectorData::Battery(BatteryData {
            gtin: Gtin::parse(VALID_GTIN).unwrap(),
            battery_chemistry: BatteryChemistry::Lfp,
            nominal_voltage_v: 3.2,
            nominal_capacity_ah: 100.0,
            expected_lifetime_cycles: 6000,
            co2e_per_unit_kg: 73.2,
            recycled_content_cobalt_pct: Some(12.0),
            recycled_content_lithium_pct: Some(6.0),
            recycled_content_nickel_pct: Some(9.0),
            state_of_health_pct: Some(100.0),
            rated_capacity_kwh: Some(0.32),
            carbon_footprint_class: Some(CarbonFootprintClass::B),
            due_diligence_url: Some("https://voltdynamics.example.com/due-diligence".into()),
            cathode_material: Some(vec![MaterialComposition {
                name: "LiFePO4".into(),
                weight_pct: 90.0,
                cas_number: Some("15365-14-7".into()),
            }]),
            anode_material: Some(vec![MaterialComposition {
                name: "graphite".into(),
                weight_pct: 95.0,
                cas_number: Some("7782-42-5".into()),
            }]),
            electrolyte_material: Some(vec![MaterialComposition {
                name: "LiPF6".into(),
                weight_pct: 12.0,
                cas_number: Some("21324-40-3".into()),
            }]),
            critical_raw_materials: Some(vec![CriticalRawMaterial {
                name: "lithium".into(),
                cas_number: Some("7439-93-2".into()),
                weight_grams: Some(820.0),
                country_of_origin: Some("AU".into()),
            }]),
            disassembly_instructions_url: Some(
                "https://voltdynamics.example.com/disassembly".into(),
            ),
            soh_methodology: Some("IEC 62660-1:2018".into()),
            operating_temp_min_c: Some(-20.0),
            operating_temp_max_c: Some(60.0),
            rated_energy_wh: Some(320.0),
            recycled_content_lead_pct: Some(0.0),
            battery_weight_kg: Some(15.5),
            battery_type: Some(BatteryType::Ev),
            round_trip_efficiency_pct: Some(94.5),
            internal_resistance_mohm: Some(0.8),
            manufacturing_date: Some(now),
            manufacturing_place: Some("DE:München".into()),
            battery_model_id: Some("VD-4680-LFP".into()),
            battery_passport_number: Some("b6c2f0a1-0000-4000-8000-000000000001".into()),
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

#[test]
fn battery_passport_serialisation_round_trip() {
    let passport = make_battery_passport();
    let json = serde_json::to_value(&passport).unwrap();
    let back: Passport = serde_json::from_value(json).unwrap();

    assert_eq!(back.id, passport.id);
    assert_eq!(back.product_name, "PowerCell EV Module 4680");
    assert_eq!(back.sector, Sector::Battery);
    assert_eq!(back.schema_version, "2.0.0");

    if let Some(SectorData::Battery(bd)) = &back.sector_data {
        assert_eq!(bd.battery_chemistry, BatteryChemistry::Lfp);
        assert_eq!(bd.expected_lifetime_cycles, 6000);
        assert_eq!(bd.battery_type, Some(BatteryType::Ev));
        assert_eq!(bd.cathode_material.as_ref().unwrap().len(), 1);
        assert_eq!(bd.critical_raw_materials.as_ref().unwrap().len(), 1);
        assert_eq!(bd.gtin.as_str(), VALID_GTIN);
    } else {
        panic!("expected BatteryData after round-trip");
    }
}

#[test]
fn battery_passport_maps_to_aas_shell() {
    let passport = make_battery_passport();
    let (shell, submodels) = build_aas_from_passport(&passport, VALID_GTIN);

    // Shell wiring: GTIN becomes the global asset id; passport + batch are
    // specific asset ids.
    assert!(shell.asset_information.global_asset_id.contains(VALID_GTIN));
    assert!(
        shell
            .asset_information
            .specific_asset_ids
            .iter()
            .any(|a| a.name == "batchId" && a.value == "LOT-2027-B-0917")
    );

    // Five core submodels + one battery sector submodel.
    assert_eq!(submodels.len(), 6);
    assert_eq!(shell.submodels.len(), 6);

    let battery = submodels
        .iter()
        .find(|s| s.id_short == "BatteryTechnicalData")
        .expect("battery sector submodel present");

    // Fully-populated battery has the 6 mandatory + many optional elements.
    assert!(
        battery.submodel_elements.len() > 15,
        "expected a rich battery submodel, got {} elements",
        battery.submodel_elements.len()
    );

    // The whole shell + submodels serialise as valid JSON.
    let shell_json = serde_json::to_value(&shell).unwrap();
    assert_eq!(shell_json["idShort"], "DigitalProductPassport");
    let submodels_json = serde_json::to_value(&submodels).unwrap();
    assert!(submodels_json.is_array());
}

#[test]
fn gs1_digital_link_parsing_for_battery() {
    let url = format!("https://id.gs1.org/01/{VALID_GTIN}/21/LOT-2027-B-0917");
    let link = DigitalLink::parse(&url).expect("should parse valid GS1 Digital Link");

    assert_eq!(link.gtin.as_str(), VALID_GTIN);
    assert_eq!(link.serial.as_deref(), Some("LOT-2027-B-0917"));

    let rebuilt = link.build();
    assert!(rebuilt.contains(VALID_GTIN));
}

#[test]
fn recycler_credential_unlocks_professional_battery_fields() {
    let passport = make_battery_passport();
    let battery_fields = serde_json::to_value(passport.sector_data.as_ref().unwrap()).unwrap();

    let policy = SectorAccessPolicy::from_catalog(&SectorCatalog::new(), "battery")
        .expect("battery in catalog");

    // ── Public tier ─────────────────────────────────────────────────────────
    let public = filter_by_access_tier(&battery_fields, &policy, AccessTier::Public);
    // Public sees the basics...
    assert!(public.filtered_data.get("gtin").is_some());
    assert!(public.filtered_data.get("batteryChemistry").is_some());
    // ...but NOT the professional-tier fields.
    assert!(
        public.filtered_data.get("dueDiligenceUrl").is_none(),
        "public must not see due-diligence url"
    );
    assert!(
        public.filtered_data.get("criticalRawMaterials").is_none(),
        "public must not see critical raw materials"
    );
    assert!(public.filtered_data.get("cathodeMaterial").is_none());
    assert!(public.filtered_data.get("sohMethodology").is_none());
    assert!(
        public
            .redacted_fields
            .contains(&"dueDiligenceUrl".to_string())
    );

    // ── Professional tier (via recycler VC) ─────────────────────────────────
    let subject = DppCredentialSubject {
        id: "did:web:cellrecycle.example.com".into(),
        name: "CellRecycle B.V.".into(),
        role: CredentialRole::Recycler,
        country: "NL".into(),
        sectors: vec!["battery".into()],
        product_categories: vec![],
    };
    let credential = CredentialBuilder::new("did:web:battery-authority.eu".into(), subject)
        .expires_in_days(365)
        .build();

    let result = verify_credential_claims(&credential, Some("battery"), Utc::now());
    assert!(result.is_valid(), "recycler credential should be valid");

    if let VerificationResult::Valid { access_tier, .. } = &result {
        assert_eq!(*access_tier, AccessTier::Professional);
        let pro = filter_by_access_tier(&battery_fields, &policy, *access_tier);
        assert!(pro.filtered_data.get("dueDiligenceUrl").is_some());
        assert!(pro.filtered_data.get("criticalRawMaterials").is_some());
        assert!(pro.filtered_data.get("cathodeMaterial").is_some());
        assert!(pro.filtered_data.get("sohMethodology").is_some());
    } else {
        panic!("expected Valid result for recycler credential");
    }
}

#[test]
fn redact_sector_data_strips_professional_fields_at_public_tier() {
    let passport = make_battery_passport();
    let catalog = SectorCatalog::new();
    let descriptor = catalog.get("battery").expect("battery in catalog");
    let data = passport.sector_data.as_ref().unwrap();

    // Public viewer: professional fields stripped, public fields retained.
    let public = redact_sector_data(data, AccessTier::Public, descriptor);
    let public_obj = public.as_object().expect("redacted data is an object");
    assert!(public_obj.contains_key("gtin"));
    assert!(!public_obj.contains_key("dueDiligenceUrl"));
    assert!(!public_obj.contains_key("criticalRawMaterials"));

    // Confidential viewer: nothing stripped (>= every required tier).
    let confidential = redact_sector_data(data, AccessTier::Confidential, descriptor);
    let conf_obj = confidential.as_object().unwrap();
    assert!(conf_obj.contains_key("dueDiligenceUrl"));
    assert!(conf_obj.contains_key("criticalRawMaterials"));
}

#[test]
fn expired_battery_credential_denied() {
    let subject = DppCredentialSubject {
        id: "did:web:expired-recycler.example.com".into(),
        name: "Expired Recycler".into(),
        role: CredentialRole::Recycler,
        country: "DE".into(),
        sectors: vec!["battery".into()],
        product_categories: vec![],
    };
    let mut credential =
        CredentialBuilder::new("did:web:battery-authority.eu".into(), subject).build();
    credential.valid_until = Utc::now() - chrono::Duration::hours(1);

    let result = verify_credential_claims(&credential, Some("battery"), Utc::now());
    assert!(
        matches!(result, VerificationResult::Expired { .. }),
        "expired battery credential must be rejected"
    );
}

#[test]
fn textile_credential_out_of_scope_for_battery() {
    let subject = DppCredentialSubject {
        id: "did:web:textile-repair.example.com".into(),
        name: "Textile Repair Co".into(),
        role: CredentialRole::AuthorisedRepairer,
        country: "FR".into(),
        sectors: vec!["textile".into()],
        product_categories: vec![],
    };
    let credential =
        CredentialBuilder::new("did:web:authority.example.com".into(), subject).build();

    let result = verify_credential_claims(&credential, Some("battery"), Utc::now());
    assert!(
        matches!(result, VerificationResult::OutOfScope { .. }),
        "textile-scoped credential must not grant battery access"
    );
}
