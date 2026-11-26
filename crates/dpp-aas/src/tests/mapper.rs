//! The textile submodels and the generic mapper.

use super::super::*;
use super::common::*;
use dpp_domain::Audience;
use dpp_domain::identifier::ProductIdentifier;
use dpp_domain::{FibreEntry, Gtin, ProductGroup, ProductGroupData, TextileData};
use serde_json::json;

#[test]
fn build_aas_textile_has_fibre_composition_collection() {
    let mut passport = minimal_passport(ProductGroup::Textile);
    passport.product_group_data = Some(ProductGroupData::Textile(Box::new(TextileData {
        product_identifier: ProductIdentifier::gs1(Gtin::parse("09506000134352").unwrap()),
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
    })));

    let (_, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
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
    let passport = minimal_passport(ProductGroup::Electronics);
    let (_, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
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
    let passport = minimal_passport(ProductGroup::Battery);
    let (_, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
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
    let passport = minimal_passport(ProductGroup::Battery);
    let (_, submodels) =
        build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
            .expect("masking");
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
    let dpp = json!({ "productGroup": "textile", "carbonFootprintKgCo2e": 8.5 });
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

// ── New product group builder tests ──────────────────────────────────────────────
