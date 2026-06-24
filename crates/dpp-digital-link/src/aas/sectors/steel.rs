use dpp_domain::domain::sector::SteelData;

use crate::aas::model::{AasSemId, AasSubmodel};
use crate::aas::property::{double_property, string_property};
use crate::aas::semantic_ids;

pub(super) fn build_steel_submodel(d: &SteelData, passport_id: &str) -> AasSubmodel {
    let route_str = serde_json::to_value(&d.production_route)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let mut elements = vec![
        string_property("gtin", &d.gtin, None, None),
        double_property(
            "co2ePerTonneSteel",
            d.co2e_per_tonne_steel,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("tCO2e/t"),
        ),
        double_property(
            "recycledScrapContentPct",
            d.recycled_scrap_content_pct,
            None,
            Some("%"),
        ),
        string_property("productCategory", &d.product_category, None, None),
        string_property("countryOfProduction", &d.country_of_production, None, None),
        string_property("productionRoute", &route_str, None, None),
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
        id: format!("urn:odal-node:dpp:{passport_id}:steel-product-data"),
        id_short: "SteelProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::STEEL_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
