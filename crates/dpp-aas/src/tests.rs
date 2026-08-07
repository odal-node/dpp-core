use super::*;
use chrono::Utc;
use dpp_domain::Audience;
use dpp_domain::{
    BatteryChemistry, BatteryData, BatteryType, CarbonFootprint, CarbonFootprintClass, FibreEntry,
    Gtin, ManufacturerInfo, MaterialEntry, Passport, PassportId, PassportStatus,
    RepairabilityScore, Sector, SectorData, TextileData, UnsoldGoodsDestination, UnsoldGoodsReason,
    UnsoldGoodsReport,
};
use serde_json::json;

fn minimal_passport(sector: Sector) -> Passport {
    Passport {
        id: PassportId::new(),
        batch_id: Some("BATCH-001".into()),
        product_name: "Test Product".into(),
        sector,
        product_category: None,
        manufacturer: ManufacturerInfo {
            name: "ACME Corp".into(),
            address: "123 Main St, Berlin, DE".into(),
            did_web_url: Some("https://acme.example.com/.well-known/did.json".into()),
        },
        materials: vec![MaterialEntry {
            name: "Aluminium".into(),
            weight_kg: 0.3,
            recycled_pct: Some(60.0),
            country_of_origin: Some("DE".into()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(12.5)),
        repairability_score: Some(RepairabilityScore::from_scalar(8.0)),
        compliance_result: None,
        lint_result: None,
        sector_data: None,
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        disclosure_signatures: Default::default(),
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
        commodity_code: None,
        operator_identifier: None,
        facility: None,
        seal: None,
    }
}

// ── Property helper tests ─────────────────────────────────────────────

#[test]
fn property_helpers() {
    let s = string_property("name", "cotton", Some("urn:eclass:0173-1#01-AAA000#001"));
    if let AasSubmodelElement::Property(p) = s {
        assert_eq!(p.id_short, "name");
        assert_eq!(p.value, "cotton");
        assert_eq!(
            p.semantic_id.unwrap().keys[0].value,
            "urn:eclass:0173-1#01-AAA000#001"
        );
    }

    let d = double_property("co2e", 8.5, None);
    if let AasSubmodelElement::Property(p) = d {
        assert_eq!(p.value, "8.5");
        assert_eq!(p.value_type, AasDataType::Double);
    }

    let b = boolean_property("available", true, None);
    if let AasSubmodelElement::Property(p) = b {
        assert_eq!(p.value, "true");
        assert_eq!(p.value_type, AasDataType::Boolean);
    }

    let i = integer_property("cycles", 3000, None);
    if let AasSubmodelElement::Property(p) = i {
        assert_eq!(p.value, "3000");
        assert_eq!(p.value_type, AasDataType::Integer);
    }
}

/// A `Property` never carries a `unit` member.
///
/// It is not part of the class — `unit` belongs to
/// `DataSpecificationIec61360`, reached through `embeddedDataSpecifications`.
/// These two tests previously asserted the opposite, which is how the defect
/// survived: the suite agreed with the code instead of with the metamodel.
///
/// Every JSON Schema accepted the bare member (none of 3.0/3.1/3.2 sets
/// `additionalProperties`), and aas-core-works' reference implementation
/// refused the whole document over it.
#[test]
fn a_property_carries_no_unit_member() {
    for prop in [
        double_property("weight", 1.5, None),
        string_property("name", "test", None),
        integer_property("cycles", 3000, None),
        boolean_property("available", true, None),
    ] {
        let json = serde_json::to_value(&prop).unwrap();
        assert!(
            json.get("unit").is_none(),
            "a Property must not serialise a `unit` member: {json}"
        );
    }

    // Not vacuous — the members that *are* part of the class still ship.
    let json = serde_json::to_value(double_property("weight", 1.5, None)).unwrap();
    assert_eq!(json["value"], "1.5");
    assert_eq!(json["valueType"], "xs:double");
    assert_eq!(json["modelType"], "Property");
}

// ── Reference element tests ───────────────────────────────────────────

#[test]
fn reference_element_round_trip() {
    let elem = AasSubmodelElement::ReferenceElement(AasReference::external(
        "repairManualUrl",
        "https://example.com/repair.pdf",
    ));
    let json = serde_json::to_value(&elem).unwrap();
    // `ReferenceElement` is the metamodel class name, and `value` is a
    // `Reference`, not a bare string.
    assert_eq!(json["modelType"], "ReferenceElement");
    assert_eq!(json["idShort"], "repairManualUrl");
    assert_eq!(json["value"]["type"], "ExternalReference");
    assert_eq!(json["value"]["keys"][0]["type"], "GlobalReference");
    assert_eq!(
        json["value"]["keys"][0]["value"],
        "https://example.com/repair.pdf"
    );
    let back: AasSubmodelElement = serde_json::from_value(json).unwrap();
    assert_eq!(elem, back);
}

// ── Shell + submodel builder tests ───────────────────────────────────

#[test]
fn build_aas_produces_five_core_submodels() {
    let passport = minimal_passport(Sector::Electronics);
    let (shell, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    assert_eq!(submodels.len(), 5);
    let id_shorts: Vec<&str> = submodels.iter().map(|s| s.id_short.as_str()).collect();
    assert!(id_shorts.contains(&"ProductIdentification"));
    assert!(id_shorts.contains(&"ManufacturerInformation"));
    assert!(id_shorts.contains(&"EnvironmentalImpact"));
    assert!(id_shorts.contains(&"MaterialComposition"));
    assert!(id_shorts.contains(&"Repairability"));
    assert_eq!(shell.submodels.len(), 5);
}

#[test]
fn shell_submodel_refs_match_submodel_ids() {
    let passport = minimal_passport(Sector::Battery);
    let (shell, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    let submodel_ids: Vec<&str> = submodels.iter().map(|s| s.id.as_str()).collect();
    for submodel_ref in &shell.submodels {
        // A shell's submodel list holds `ModelReference`s whose single key
        // names the target Submodel by id.
        assert_eq!(submodel_ref.ref_type, "ModelReference");
        let key = submodel_ref.keys.first().expect("a reference has one key");
        assert_eq!(key.key_type, "Submodel");
        assert!(
            submodel_ids.contains(&key.value.as_str()),
            "shell ref {} not found in submodels",
            key.value
        );
    }
}

#[test]
fn shell_has_correct_asset_information() {
    let passport = minimal_passport(Sector::Textile);
    let (shell, _) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    assert_eq!(
        shell.asset_information.global_asset_id,
        "urn:odal-node:product:09506000134352"
    );
    let names: Vec<&str> = shell
        .asset_information
        .specific_asset_ids
        .iter()
        .map(|id| id.name.as_str())
        .collect();
    assert!(names.contains(&"gtin"));
    assert!(names.contains(&"serialId"));
    assert!(
        names.contains(&"batchId"),
        "batch_id should appear when set"
    );
}

#[test]
fn shell_id_contains_passport_id() {
    let passport = minimal_passport(Sector::Battery);
    let id_str = passport.id.to_string();
    let (shell, _) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    assert!(shell.id.contains(&id_str));
}

fn battery_data_with_due_diligence() -> BatteryData {
    BatteryData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: 3000,
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: None,
        recycled_content_lithium_pct: Some(12.5),
        recycled_content_nickel_pct: None,
        state_of_health_pct: Some(95.0),
        state_of_health: None,
        expected_lifetime: None,
        recycled_content_reporting_year: None,
        rated_capacity_kwh: Some(32.0),
        carbon_footprint_class: Some(CarbonFootprintClass::new("B").expect("valid label")),
        carbon_footprint_class_ruleset_id: Some("test-cfb-classes".into()),
        carbon_footprint_class_ruleset_version: Some("0.0.0-test".into()),
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        cathode_material: None,
        anode_material: None,
        electrolyte_material: None,
        critical_raw_materials: None,
        disassembly_instructions_url: None,
        soh_methodology: None,
        operating_temp_min_c: Some(-20.0),
        operating_temp_max_c: Some(60.0),
        rated_energy_wh: None,
        recycled_content_lead_pct: None,
        battery_weight_kg: Some(8.5),
        battery_type: Some(BatteryType::Ev),
        round_trip_efficiency_pct: Some(94.0),
        internal_resistance_mohm: Some(3.2),
        placed_on_market_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 15),
        manufacturing_date: None,
        manufacturing_place: None,
        battery_model_id: None,
        battery_passport_number: None,
    }
}

#[test]
fn build_aas_with_battery_sector_data_adds_sixth_submodel() {
    let mut passport = minimal_passport(Sector::Battery);
    passport.sector_data = Some(SectorData::Battery(battery_data_with_due_diligence()));

    let (shell, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    assert_eq!(
        submodels.len(),
        6,
        "battery sector data should add a 6th submodel"
    );
    assert_eq!(shell.submodels.len(), 6);

    let battery_sub = submodels
        .iter()
        .find(|s| s.id_short == "BatteryTechnicalData");
    assert!(
        battery_sub.is_some(),
        "BatteryTechnicalData submodel missing"
    );

    let battery_sub = battery_sub.unwrap();
    let has_chemistry = battery_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "batteryChemistry",
        _ => false,
    });
    assert!(has_chemistry, "batteryChemistry property missing");

    let has_co2e = battery_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "co2ePerUnitKg",
        _ => false,
    });
    assert!(has_co2e, "co2ePerUnitKg property missing");

    // `dueDiligenceUrl` is `restricted` in the battery catalog, so it must NOT
    // reach a public projection. This assertion used to be its inverse — the
    // mappers emitted it to everyone, and the test locked that in. It is kept
    // pointing the other way as the regression marker for that defect.
    let has_due_diligence_ref = battery_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::ReferenceElement(r) => r.id_short == "dueDiligenceUrl",
        _ => false,
    });
    assert!(
        !has_due_diligence_ref,
        "restricted field dueDiligenceUrl leaked into a public AAS projection"
    );
}

/// The same field is present for a caller entitled to it — proof the masking is
/// per-audience rather than a blanket strip of everything non-public.
#[test]
fn restricted_audience_receives_the_restricted_battery_field() {
    let mut passport = minimal_passport(Sector::Battery);
    passport.sector_data = Some(SectorData::Battery(battery_data_with_due_diligence()));

    let (_, submodels) = build_aas_from_passport(
        &passport,
        "09506000134352",
        dpp_domain::Audience::LegitimateInterest,
    )
    .expect("buildable");

    let battery_sub = submodels
        .iter()
        .find(|s| s.id_short == "BatteryTechnicalData")
        .expect("battery submodel present");

    assert!(
        battery_sub.submodel_elements.iter().any(|e| match e {
            AasSubmodelElement::ReferenceElement(r) => r.id_short == "dueDiligenceUrl",
            _ => false,
        }),
        "a legitimate-interest caller must still receive dueDiligenceUrl"
    );
}

#[test]
fn build_aas_textile_has_fibre_composition_collection() {
    let mut passport = minimal_passport(Sector::Textile);
    passport.sector_data = Some(SectorData::Textile(TextileData {
        gtin: Gtin::parse("09506000134352").unwrap(),
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
        care_instructions: "Machine wash 30°C".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        recycled_content_pct: Some(30.0),
        carbon_footprint_kg_co2e: Some(8.2),
        water_use_litres: None,
        microplastic_shedding_mg_per_wash: None,
        repair_score: Some(6.5),
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
    }));

    let (_, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    let textile_sub = submodels
        .iter()
        .find(|s| s.id_short == "TextileMaterialDeclaration")
        .expect("TextileMaterialDeclaration missing");

    let fibre_coll = textile_sub.submodel_elements.iter().find(|e| match e {
        AasSubmodelElement::SubmodelElementCollection(c) => c.id_short == "fibreComposition",
        _ => false,
    });
    assert!(fibre_coll.is_some(), "fibreComposition collection missing");

    if let Some(AasSubmodelElement::SubmodelElementCollection(coll)) = fibre_coll {
        assert_eq!(coll.value.len(), 2, "expected 2 fibre entries");
        // First fibre entry should contain countryOfOrigin
        if let AasSubmodelElement::SubmodelElementCollection(fibre0) = &coll.value[0] {
            let has_origin = fibre0.value.iter().any(|e| match e {
                AasSubmodelElement::Property(p) => p.id_short == "countryOfOrigin",
                _ => false,
            });
            assert!(has_origin, "countryOfOrigin missing from first fibre entry");
        }
    }
}

#[test]
fn material_composition_entries_have_unit() {
    let passport = minimal_passport(Sector::Electronics);
    let (_, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    let mat_sub = submodels
        .iter()
        .find(|s| s.id_short == "MaterialComposition")
        .unwrap();

    if let AasSubmodelElement::SubmodelElementCollection(mat0) = &mat_sub.submodel_elements[0] {
        let weight_prop = mat0.value.iter().find_map(|e| match e {
            AasSubmodelElement::Property(p) if p.id_short == "weightKg" => Some(p),
            _ => None,
        });
        assert!(weight_prop.is_some());
        assert_eq!(weight_prop.unwrap().value, "0.3");
    } else {
        panic!("expected material_0 collection");
    }
}

#[test]
fn environmental_impact_co2e_has_unit() {
    let passport = minimal_passport(Sector::Battery);
    let (_, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    let env_sub = submodels
        .iter()
        .find(|s| s.id_short == "EnvironmentalImpact")
        .unwrap();
    let co2e_prop = env_sub.submodel_elements.iter().find_map(|e| match e {
        AasSubmodelElement::Property(p) if p.id_short == "co2ePerUnit" => Some(p),
        _ => None,
    });
    assert!(co2e_prop.is_some());
    assert_eq!(co2e_prop.unwrap().value_type, AasDataType::Double);
}

#[test]
fn manufacturer_submodel_has_did_reference() {
    let passport = minimal_passport(Sector::Battery);
    let (_, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    let mfr_sub = submodels
        .iter()
        .find(|s| s.id_short == "ManufacturerInformation")
        .unwrap();
    let has_did_ref = mfr_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::ReferenceElement(r) => r.id_short == "didWebUrl",
        _ => false,
    });
    assert!(
        has_did_ref,
        "didWebUrl Reference element missing from ManufacturerInformation"
    );
}

// ── Generic mapper tests (unchanged behaviour) ────────────────────────

#[test]
fn map_simple_textile_data() {
    let dpp = json!({
        "countryOfManufacturing": "BD",
        "carbonFootprintKgCo2e": 8.5,
        "durabilityScore": 7.5,
        "sparePartsAvailable": true
    });
    let submodel = map_dpp_to_aas_submodel("urn:odal-node:dpp:test-001", &dpp);
    assert_eq!(submodel.id_short, "DigitalProductPassport");
    assert_eq!(submodel.submodel_elements.len(), 4);

    let country = submodel.submodel_elements.iter().find(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "countryOfManufacturing",
        _ => false,
    });
    assert!(country.is_some());
    if let Some(AasSubmodelElement::Property(p)) = country {
        assert_eq!(p.value, "BD");
        assert_eq!(p.value_type, AasDataType::String);
    }
}

#[test]
fn map_nested_object_becomes_collection() {
    let dpp = json!({ "manufacturer": { "name": "EcoTextile GmbH", "country": "DE" } });
    let submodel = map_dpp_to_aas_submodel("urn:test", &dpp);
    assert_eq!(submodel.submodel_elements.len(), 1);
    if let AasSubmodelElement::SubmodelElementCollection(col) = &submodel.submodel_elements[0] {
        assert_eq!(col.id_short, "manufacturer");
        assert_eq!(col.value.len(), 2);
    } else {
        panic!("expected SubmodelElementCollection");
    }
}

#[test]
fn map_array_becomes_indexed_collection() {
    let dpp = json!({
        "fibreComposition": [
            { "fibre": "cotton", "pct": 70.0 },
            { "fibre": "polyester", "pct": 30.0 }
        ]
    });
    let submodel = map_dpp_to_aas_submodel("urn:test", &dpp);
    if let AasSubmodelElement::SubmodelElementCollection(col) = &submodel.submodel_elements[0] {
        assert_eq!(col.id_short, "fibreComposition");
        assert_eq!(col.value.len(), 2);
        // Items use semantic "item_{i}" idShorts, not "{key}_{i}" synthetics.
        if let AasSubmodelElement::SubmodelElementCollection(item) = &col.value[0] {
            assert_eq!(item.id_short, "item_0");
        } else {
            panic!("expected collection for array item");
        }
    } else {
        panic!("expected collection for array");
    }
}

#[test]
fn submodel_round_trip() {
    let dpp = json!({ "sector": "textile", "carbonFootprintKgCo2e": 8.5 });
    let submodel = map_dpp_to_aas_submodel("urn:test", &dpp);
    let json = serde_json::to_value(&submodel).unwrap();
    let back: AasSubmodel = serde_json::from_value(json).unwrap();
    assert_eq!(submodel, back);
}

#[test]
fn empty_input_produces_empty_submodel() {
    let submodel = map_dpp_to_aas_submodel("urn:test", &json!({}));
    assert!(submodel.submodel_elements.is_empty());
}

#[test]
fn non_object_input_produces_empty_submodel() {
    let submodel = map_dpp_to_aas_submodel("urn:test", &json!("not an object"));
    assert!(submodel.submodel_elements.is_empty());
}

// ── New sector builder tests ──────────────────────────────────────────────

#[test]
fn build_aas_unsold_goods_produces_sector_submodel() {
    let mut passport = minimal_passport(Sector::UnsoldGoods);
    passport.sector_data = Some(SectorData::UnsoldGoods(UnsoldGoodsReport {
        reporting_period: "2026-Q2".into(),
        volume_kg: 1500.0,
        product_category: "apparel".into(),
        reason: UnsoldGoodsReason::EndOfSeason,
        destination: UnsoldGoodsDestination::Donation,
        destruction_justification: None,
        country_of_disposal: "DE".into(),
        operator_name: Some("GoodWill e.V.".into()),
    }));
    let (_, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    let sub = submodels.iter().find(|s| s.id_short == "UnsoldGoods");
    assert!(sub.is_some(), "UnsoldGoods submodel missing");
    let has_volume = sub.unwrap().submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "volumeKg",
        _ => false,
    });
    assert!(has_volume, "volumeKg property missing");
}

/// A sector whose typed mapper was removed still ships its data — the
/// projection loses its *shape*, not its *content*.
///
/// This is the property that makes removing a typed lane safe rather than
/// lossy: the generic builder renders the variant's serialised fields, minus
/// the `sector` discriminant, so a consumer still receives every value. What it
/// no longer receives is a submodel named for a template that no standards body
/// has ratified, which was never a claim we could support.
#[test]
fn a_sector_without_a_typed_mapper_still_carries_its_data() {
    let mut passport = minimal_passport(Sector::Steel);
    passport.sector_data = Some(SectorData::Steel(dpp_domain::SteelData {
        gtin: dpp_domain::Gtin::parse("09506000134352").expect("valid gtin"),
        co2e_per_tonne_steel: 1.8,
        recycled_scrap_content_pct: 62.0,
        product_category: "flat".into(),
        country_of_origin: "DE".into(),
        production_route: dpp_domain::ProductionRoute::ElectricArc,
        annual_production_tonnes: None,
    }));

    let (_, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("masking");
    let sector_submodel = submodels
        .iter()
        .find(|s| s.id_short == "SectorData")
        .expect("a provisional sector renders through the generic builder");

    assert!(
        !sector_submodel.submodel_elements.is_empty(),
        "the generic projection must carry the sector's fields, not drop them"
    );
    assert!(
        sector_submodel.semantic_id.is_none(),
        "a generic projection asserts no semanticId — there is no ratified \
         template for it to name"
    );
}

// ─── Environment ──────────────────────────────────────────────────────────────

#[test]
fn environment_carries_the_shell_and_every_submodel() {
    let mut passport = minimal_passport(Sector::Battery);
    passport.sector_data = Some(SectorData::Battery(battery_data_with_due_diligence()));

    let env = build_aas_environment(&passport, "09506000134352", Audience::Public)
        .expect("environment is buildable");

    assert_eq!(env.asset_administration_shells.len(), 1);
    assert_eq!(env.submodels.len(), 6);
    assert!(
        env.concept_descriptions.is_empty(),
        "this crate coins no concept descriptions"
    );

    // The envelope must carry exactly what the pair form carries — the whole
    // point of one builder is that the two cannot disagree.
    let (shell, submodels) =
        build_aas_from_passport(&passport, "09506000134352", Audience::Public).expect("pair form");
    assert_eq!(env.asset_administration_shells[0], shell);
    assert_eq!(env.submodels, submodels);
}

#[test]
fn environment_is_masked_for_its_audience() {
    let mut passport = minimal_passport(Sector::Battery);
    passport.sector_data = Some(SectorData::Battery(battery_data_with_due_diligence()));

    let public = build_aas_environment(&passport, "09506000134352", Audience::Public)
        .expect("public environment");
    let serialised = serde_json::to_string(&public).expect("serialises");

    // `dueDiligenceUrl` is `restricted` in the battery catalog. The envelope
    // delegates to the masked builder, so it must not appear at any depth —
    // an envelope that assembled its own content could reintroduce it.
    assert!(
        !serialised.contains("dueDiligenceUrl"),
        "restricted field leaked into a public AAS Environment"
    );

    let restricted =
        build_aas_environment(&passport, "09506000134352", Audience::LegitimateInterest)
            .expect("restricted environment");
    assert!(
        serde_json::to_string(&restricted)
            .expect("serialises")
            .contains("dueDiligenceUrl"),
        "a legitimate-interest caller must still receive it"
    );
}

#[test]
fn environment_serialises_with_idta_field_names() {
    let passport = minimal_passport(Sector::Battery);
    let env =
        build_aas_environment(&passport, "09506000134352", Audience::Public).expect("buildable");
    let value = serde_json::to_value(&env).expect("serialises");

    for key in ["assetAdministrationShells", "submodels"] {
        assert!(
            value.get(key).is_some(),
            "Environment must carry '{key}' at the top level"
        );
    }

    // `conceptDescriptions` is absent rather than `[]`. The schema constrains
    // it to `minItems: 1`, so emitting the empty array — which this crate
    // always would, coining no concept descriptions — makes the whole document
    // invalid. Absent is both valid and the honest encoding of "none".
    assert!(
        value.get("conceptDescriptions").is_none(),
        "an empty conceptDescriptions array must not reach the wire"
    );
}
