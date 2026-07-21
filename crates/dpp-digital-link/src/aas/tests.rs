use super::*;
use chrono::Utc;
use dpp_domain::{
    AluminiumData, BatteryChemistry, BatteryData, BatteryType, CarbonFootprint,
    CarbonFootprintClass, ConstructionData, DetergentData, FibreEntry, FurnitureData, Gtin,
    ManufacturerInfo, MaterialEntry, Passport, PassportId, PassportStatus, ProductionRoute,
    RepairabilityScore, Sector, SectorData, SteelData, SurfactantEntry, TextileData, ToyData,
    TyreData, UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport,
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
            origin_country: Some("DE".into()),
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
    }
}

// ── Property helper tests ─────────────────────────────────────────────

#[test]
fn property_helpers() {
    let s = string_property(
        "name",
        "cotton",
        Some("urn:eclass:0173-1#01-AAA000#001"),
        None,
    );
    if let AasSubmodelElement::Property(p) = s {
        assert_eq!(p.id_short, "name");
        assert_eq!(p.value, "cotton");
        assert_eq!(
            p.semantic_id.unwrap().keys[0].value,
            "urn:eclass:0173-1#01-AAA000#001"
        );
        assert!(p.unit.is_none());
    }

    let d = double_property("co2e", 8.5, None, Some("kgCO2e"));
    if let AasSubmodelElement::Property(p) = d {
        assert_eq!(p.value, "8.5");
        assert_eq!(p.value_type, AasDataType::Double);
        assert_eq!(p.unit.as_deref(), Some("kgCO2e"));
    }

    let b = boolean_property("available", true, None, None);
    if let AasSubmodelElement::Property(p) = b {
        assert_eq!(p.value, "true");
        assert_eq!(p.value_type, AasDataType::Boolean);
        assert!(p.unit.is_none());
    }

    let i = integer_property("cycles", 3000, None, None);
    if let AasSubmodelElement::Property(p) = i {
        assert_eq!(p.value, "3000");
        assert_eq!(p.value_type, AasDataType::Integer);
    }
}

#[test]
fn property_with_unit_serialises() {
    let prop = double_property("weight", 1.5, None, Some("kg"));
    let json = serde_json::to_value(&prop).unwrap();
    assert_eq!(json["unit"], "kg");
    assert_eq!(json["value"], "1.5");
}

#[test]
fn property_without_unit_omits_field() {
    let prop = string_property("name", "test", None, None);
    let json = serde_json::to_value(&prop).unwrap();
    assert!(
        json.get("unit").is_none(),
        "unit should be absent when None"
    );
}

// ── Reference element tests ───────────────────────────────────────────

#[test]
fn reference_element_round_trip() {
    let elem = AasSubmodelElement::Reference(AasReference {
        id_short: "repairManualUrl".into(),
        value: "https://example.com/repair.pdf".into(),
        semantic_id: None,
    });
    let json = serde_json::to_value(&elem).unwrap();
    assert_eq!(json["modelType"], "Reference");
    assert_eq!(json["idShort"], "repairManualUrl");
    assert_eq!(json["value"], "https://example.com/repair.pdf");
    let back: AasSubmodelElement = serde_json::from_value(json).unwrap();
    assert_eq!(elem, back);
}

// ── Shell + submodel builder tests ───────────────────────────────────

#[test]
fn build_aas_produces_five_core_submodels() {
    let passport = minimal_passport(Sector::Electronics);
    let (shell, submodels) = build_aas_from_passport(&passport, "09506000134352");
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
    let (shell, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let submodel_ids: Vec<&str> = submodels.iter().map(|s| s.id.as_str()).collect();
    for submodel_ref in &shell.submodels {
        assert!(
            submodel_ids.contains(&submodel_ref.id.as_str()),
            "shell ref {} not found in submodels",
            submodel_ref.id
        );
    }
}

#[test]
fn shell_has_correct_asset_information() {
    let passport = minimal_passport(Sector::Textile);
    let (shell, _) = build_aas_from_passport(&passport, "09506000134352");
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
    let (shell, _) = build_aas_from_passport(&passport, "09506000134352");
    assert!(shell.id.contains(&id_str));
}

#[test]
fn build_aas_with_battery_sector_data_adds_sixth_submodel() {
    let mut passport = minimal_passport(Sector::Battery);
    passport.sector_data = Some(SectorData::Battery(BatteryData {
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
        rated_capacity_kwh: Some(32.0),
        carbon_footprint_class: Some(CarbonFootprintClass::B),
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
        manufacturing_date: None,
        manufacturing_place: None,
        battery_model_id: None,
        battery_passport_number: None,
    }));

    let (shell, submodels) = build_aas_from_passport(&passport, "09506000134352");
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
        AasSubmodelElement::Property(p) => {
            p.id_short == "co2ePerUnitKg" && p.unit.as_deref() == Some("kgCO2e")
        }
        _ => false,
    });
    assert!(has_co2e, "co2ePerUnitKg with unit kgCO2e missing");

    let has_due_diligence_ref = battery_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Reference(r) => r.id_short == "dueDigiligenceUrl",
        _ => false,
    });
    assert!(
        has_due_diligence_ref,
        "dueDigiligenceUrl Reference element missing"
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

    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
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
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
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
        assert_eq!(weight_prop.unwrap().unit.as_deref(), Some("kg"));
    } else {
        panic!("expected material_0 collection");
    }
}

#[test]
fn environmental_impact_co2e_has_unit() {
    let passport = minimal_passport(Sector::Battery);
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let env_sub = submodels
        .iter()
        .find(|s| s.id_short == "EnvironmentalImpact")
        .unwrap();
    let co2e_prop = env_sub.submodel_elements.iter().find_map(|e| match e {
        AasSubmodelElement::Property(p) if p.id_short == "co2ePerUnit" => Some(p),
        _ => None,
    });
    assert!(co2e_prop.is_some());
    assert_eq!(co2e_prop.unwrap().unit.as_deref(), Some("kgCO2e"));
}

#[test]
fn manufacturer_submodel_has_did_reference() {
    let passport = minimal_passport(Sector::Battery);
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let mfr_sub = submodels
        .iter()
        .find(|s| s.id_short == "ManufacturerInformation")
        .unwrap();
    let has_did_ref = mfr_sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Reference(r) => r.id_short == "didWebUrl",
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
fn build_aas_steel_produces_sector_submodel() {
    let mut passport = minimal_passport(Sector::Steel);
    passport.sector_data = Some(SectorData::Steel(SteelData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        co2e_per_tonne_steel: 1.8,
        recycled_scrap_content_pct: 30.0,
        product_category: "flat".into(),
        country_of_origin: "DE".into(),
        production_route: ProductionRoute::ElectricArc,
        annual_production_tonnes: Some(50000.0),
    }));
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels.iter().find(|s| s.id_short == "SteelProductData");
    assert!(sub.is_some(), "SteelProductData submodel missing");
    let sub = sub.unwrap();
    assert!(sub.semantic_id.is_some());
    let has_route = sub.submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "productionRoute",
        _ => false,
    });
    assert!(has_route, "productionRoute property missing");
}

#[test]
fn build_aas_construction_produces_sector_submodel() {
    let mut passport = minimal_passport(Sector::Construction);
    passport.sector_data = Some(SectorData::Construction(ConstructionData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        product_family: "cement".into(),
        country_of_origin: "DE".into(),
        co2e_per_functional_unit_kg: 0.83,
        functional_unit: "per tonne".into(),
        recycled_content_pct: None,
        epd_url: Some("https://example.com/epd.pdf".into()),
        ce_marking: Some(true),
    }));
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels
        .iter()
        .find(|s| s.id_short == "ConstructionProductData");
    assert!(sub.is_some(), "ConstructionProductData submodel missing");
    let has_epd_ref = sub.unwrap().submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Reference(r) => r.id_short == "epdUrl",
        _ => false,
    });
    assert!(has_epd_ref, "epdUrl Reference missing");
}

#[test]
fn build_aas_tyre_produces_sector_submodel() {
    let mut passport = minimal_passport(Sector::Tyre);
    passport.sector_data = Some(SectorData::Tyre(TyreData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        tyre_class: "C1".into(),
        fuel_efficiency_class: "B".into(),
        wet_grip_class: "A".into(),
        external_rolling_noise_db: 68.0,
        noise_performance_class: Some("A".into()),
        rolling_resistance_n_per_kn: Some(6.5),
        recycled_rubber_pct: None,
        co2e_per_tyre_kg: Some(12.0),
    }));
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels.iter().find(|s| s.id_short == "TyreProductData");
    assert!(sub.is_some(), "TyreProductData submodel missing");
}

#[test]
fn build_aas_toy_produces_sector_submodel() {
    let mut passport = minimal_passport(Sector::Toy);
    passport.sector_data = Some(SectorData::Toy(ToyData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        age_group: "3-6".into(),
        primary_material: "plastic".into(),
        ce_marking: true,
        country_of_origin: "CN".into(),
        svhc_substances: None,
        contains_battery: Some(false),
        repairability_info: None,
    }));
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels.iter().find(|s| s.id_short == "ToyProductData");
    assert!(sub.is_some(), "ToyProductData submodel missing");
}

#[test]
fn build_aas_aluminium_produces_sector_submodel() {
    let mut passport = minimal_passport(Sector::Aluminium);
    passport.sector_data = Some(SectorData::Aluminium(AluminiumData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        alloy_grade: "6xxx".into(),
        production_route: ProductionRoute::SecondaryRecycled,
        co2e_per_tonne_kg: 2.1,
        recycled_content_pct: 75.0,
        country_of_origin: "NO".into(),
        annual_production_tonnes: None,
    }));
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels
        .iter()
        .find(|s| s.id_short == "AluminiumProductData");
    assert!(sub.is_some(), "AluminiumProductData submodel missing");
}

#[test]
fn build_aas_furniture_produces_sector_submodel() {
    let mut passport = minimal_passport(Sector::Furniture);
    passport.sector_data = Some(SectorData::Furniture(FurnitureData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        product_type: "chair".into(),
        primary_material: "solid-wood".into(),
        country_of_origin: "PT".into(),
        co2e_per_unit_kg: Some(18.5),
        recycled_content_pct: Some(20.0),
        repairability_score: Some(7.5),
        svhc_substances: None,
        disassembly_instructions_url: None,
        end_of_life_instructions: Some("Separate wood from metal. Recycle metal.".into()),
    }));
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels
        .iter()
        .find(|s| s.id_short == "FurnitureProductData");
    assert!(sub.is_some(), "FurnitureProductData submodel missing");
}

#[test]
fn build_aas_detergent_produces_surfactant_collection() {
    let mut passport = minimal_passport(Sector::Detergent);
    passport.sector_data = Some(SectorData::Detergent(DetergentData {
        gtin: Gtin::parse("09506000134352").unwrap(),
        product_type: "laundry".into(),
        format: "liquid".into(),
        surfactants: vec![SurfactantEntry {
            name: "Linear Alkylbenzene Sulfonate".into(),
            biodegradable: true,
            concentration_band: "15-30%".into(),
            cas_number: Some("68411-30-3".into()),
        }],
        country_of_origin: "DE".into(),
        co2e_per_unit_kg: Some(0.35),
        packaging_recyclable: Some(true),
        recommended_dosage_ml: Some(55.0),
        biodegradable: Some(true),
    }));
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels
        .iter()
        .find(|s| s.id_short == "DetergentProductData");
    assert!(sub.is_some(), "DetergentProductData submodel missing");
    let has_surfactants = sub.unwrap().submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::SubmodelElementCollection(c) => c.id_short == "surfactants",
        _ => false,
    });
    assert!(has_surfactants, "surfactants collection missing");
}

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
    let (_, submodels) = build_aas_from_passport(&passport, "09506000134352");
    let sub = submodels.iter().find(|s| s.id_short == "UnsoldGoods");
    assert!(sub.is_some(), "UnsoldGoods submodel missing");
    let has_volume = sub.unwrap().submodel_elements.iter().any(|e| match e {
        AasSubmodelElement::Property(p) => p.id_short == "volumeKg",
        _ => false,
    });
    assert!(has_volume, "volumeKg property missing");
}
