use dpp_domain::domain::sector::AluminiumData;

use crate::aas::model::{AasSemId, AasSubmodel};
use crate::aas::property::{double_property, enum_wire_str, string_property};
use crate::aas::semantic_ids;

pub(super) fn build_aluminium_submodel(d: &AluminiumData, passport_id: &str) -> AasSubmodel {
    let route_str = enum_wire_str(&d.production_route);
    let mut elements = vec![
        string_property("gtin", &d.gtin, None, None),
        string_property("alloyGrade", &d.alloy_grade, None, None),
        string_property("productionRoute", &route_str, None, None),
        double_property(
            "co2ePerTonneKg",
            d.co2e_per_tonne_kg,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("kgCO2e/t"),
        ),
        double_property(
            "recycledContentPct",
            d.recycled_content_pct,
            None,
            Some("%"),
        ),
        string_property("countryOfProduction", &d.country_of_production, None, None),
    ];
    if let Some(v) = d.annual_production_tonnes {
        elements.push(double_property(
            "annualProductionTonnes",
            v,
            None,
            Some("t"),
        ));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:aluminium-product-data"),
        id_short: "AluminiumProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::ALUMINIUM_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
