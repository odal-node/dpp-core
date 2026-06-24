use dpp_domain::domain::sector::DetergentData;

use crate::aas::model::{AasCollection, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::aas::property::{boolean_property, double_property, string_property};
use crate::aas::semantic_ids;

pub(super) fn build_detergent_submodel(d: &DetergentData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("gtin", &d.gtin, None, None),
        string_property("productType", &d.product_type, None, None),
        string_property("format", &d.format, None, None),
        string_property(
            "countryOfManufacture",
            &d.country_of_manufacture,
            None,
            None,
        ),
    ];

    let surfactant_items = d
        .surfactants
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut elems = vec![
                string_property("name", &s.name, None, None),
                boolean_property("biodegradable", s.biodegradable, None, None),
                string_property("concentrationBand", &s.concentration_band, None, None),
            ];
            if let Some(ref cas) = s.cas_number {
                elems.push(string_property("casNumber", cas, None, None));
            }
            AasSubmodelElement::SubmodelElementCollection(AasCollection {
                id_short: format!("surfactant_{i}"),
                value: elems,
                semantic_id: None,
            })
        })
        .collect();
    elements.push(AasSubmodelElement::SubmodelElementCollection(
        AasCollection {
            id_short: "surfactants".into(),
            value: surfactant_items,
            semantic_id: None,
        },
    ));

    if let Some(v) = d.co2e_per_unit_kg {
        elements.push(double_property(
            "co2ePerUnitKg",
            v,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("kgCO2e"),
        ));
    }
    if let Some(v) = d.packaging_recyclable {
        elements.push(boolean_property("packagingRecyclable", v, None, None));
    }
    if let Some(v) = d.recommended_dosage_ml {
        elements.push(double_property("recommendedDosageMl", v, None, Some("mL")));
    }
    if let Some(v) = d.biodegradable {
        elements.push(boolean_property("biodegradable", v, None, None));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:detergent-product-data"),
        id_short: "DetergentProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::DETERGENT_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
