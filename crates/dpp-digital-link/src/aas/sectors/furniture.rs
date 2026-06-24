use dpp_domain::domain::sector::FurnitureData;

use crate::aas::model::{AasCollection, AasReference, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::aas::property::{double_property, string_property};
use crate::aas::semantic_ids;

pub(super) fn build_furniture_submodel(d: &FurnitureData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("gtin", &d.gtin, None, None),
        string_property("productType", &d.product_type, None, None),
        string_property("primaryMaterial", &d.primary_material, None, None),
        string_property(
            "countryOfManufacture",
            &d.country_of_manufacture,
            None,
            None,
        ),
    ];
    if let Some(v) = d.co2e_per_unit_kg {
        elements.push(double_property(
            "co2ePerUnitKg",
            v,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("kgCO2e"),
        ));
    }
    if let Some(v) = d.recycled_content_pct {
        elements.push(double_property("recycledContentPct", v, None, Some("%")));
    }
    if let Some(v) = d.repairability_score {
        elements.push(double_property(
            "repairabilityScore",
            v,
            Some(semantic_ids::REPAIRABILITY),
            Some("index 0-10"),
        ));
    }
    if let Some(ref url) = d.disassembly_instructions_url {
        elements.push(AasSubmodelElement::Reference(AasReference {
            id_short: "disassemblyInstructionsUrl".into(),
            value: url.clone(),
            semantic_id: None,
        }));
    }
    if let Some(ref v) = d.end_of_life_instructions {
        elements.push(string_property("endOfLifeInstructions", v, None, None));
    }
    if let Some(ref svhcs) = d.svhc_substances {
        let items = svhcs
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let mut elems = vec![
                    string_property("casNumber", &s.cas_number, None, None),
                    string_property("substanceName", &s.substance_name, None, None),
                    double_property("concentrationPct", s.concentration_pct, None, Some("%")),
                ];
                if let Some(ref loc) = s.location_in_product {
                    elems.push(string_property("locationInProduct", loc, None, None));
                }
                AasSubmodelElement::SubmodelElementCollection(AasCollection {
                    id_short: format!("svhc_{i}"),
                    value: elems,
                    semantic_id: None,
                })
            })
            .collect();
        elements.push(AasSubmodelElement::SubmodelElementCollection(
            AasCollection {
                id_short: "svhcSubstances".into(),
                value: items,
                semantic_id: None,
            },
        ));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:furniture-product-data"),
        id_short: "FurnitureProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::FURNITURE_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
