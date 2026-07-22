use dpp_domain::domain::sector::ConstructionData;

use crate::aas::model::{AasReference, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::aas::property::{boolean_property, double_property, string_property};
use crate::aas::semantic_ids;

pub(super) fn build_construction_submodel(d: &ConstructionData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("gtin", d.gtin.as_str(), None, None),
        string_property("productFamily", &d.product_family, None, None),
        string_property("countryOfOrigin", &d.country_of_origin, None, None),
        double_property(
            "co2ePerFunctionalUnitKg",
            d.co2e_per_functional_unit_kg,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("kgCO2e"),
        ),
        string_property("functionalUnit", &d.functional_unit, None, None),
    ];
    if let Some(v) = d.recycled_content_pct {
        elements.push(double_property("recycledContentPct", v, None, Some("%")));
    }
    if let Some(ref url) = d.epd_url {
        elements.push(AasSubmodelElement::Reference(AasReference {
            id_short: "epdUrl".into(),
            value: url.clone(),
            semantic_id: None,
        }));
    }
    if let Some(v) = d.ce_marking {
        elements.push(boolean_property("ceMarking", v, None, None));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:construction-product-data"),
        id_short: "ConstructionProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::CONSTRUCTION_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
