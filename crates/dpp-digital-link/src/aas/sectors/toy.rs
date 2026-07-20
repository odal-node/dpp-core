use dpp_domain::domain::sector::ToyData;

use crate::aas::model::{AasCollection, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::aas::property::{boolean_property, string_property, svhc_substance_element};
use crate::aas::semantic_ids;

pub(super) fn build_toy_submodel(d: &ToyData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("gtin", &d.gtin, None, None),
        string_property("ageGroup", &d.age_group, None, None),
        string_property("primaryMaterial", &d.primary_material, None, None),
        boolean_property("ceMarking", d.ce_marking, None, None),
        string_property(
            "countryOfManufacture",
            &d.country_of_manufacture,
            None,
            None,
        ),
    ];
    if let Some(v) = d.contains_battery {
        elements.push(boolean_property("containsBattery", v, None, None));
    }
    if let Some(ref v) = d.repairability_info {
        elements.push(string_property("repairabilityInfo", v, None, None));
    }
    if let Some(ref svhcs) = d.svhc_substances {
        let items = svhcs
            .iter()
            .enumerate()
            .map(|(i, s)| {
                AasSubmodelElement::SubmodelElementCollection(svhc_substance_element(
                    i,
                    &s.cas_number,
                    &s.substance_name,
                    s.concentration_pct,
                    s.location_in_product.as_deref(),
                ))
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
        id: format!("urn:odal-node:dpp:{passport_id}:toy-product-data"),
        id_short: "ToyProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::TOY_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
