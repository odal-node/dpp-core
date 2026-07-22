use dpp_domain::domain::sector::TyreData;

use crate::aas::model::{AasSemId, AasSubmodel};
use crate::aas::property::{double_property, string_property};
use crate::aas::semantic_ids;

pub(super) fn build_tyre_submodel(d: &TyreData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("gtin", d.gtin.as_str(), None, None),
        string_property("tyreClass", &d.tyre_class, None, None),
        string_property("fuelEfficiencyClass", &d.fuel_efficiency_class, None, None),
        string_property("wetGripClass", &d.wet_grip_class, None, None),
        double_property(
            "externalRollingNoiseDb",
            d.external_rolling_noise_db,
            None,
            Some("dB"),
        ),
    ];
    if let Some(ref v) = d.noise_performance_class {
        elements.push(string_property("noisePerformanceClass", v, None, None));
    }
    if let Some(v) = d.rolling_resistance_n_per_kn {
        elements.push(double_property(
            "rollingResistanceNPerKn",
            v,
            None,
            Some("N/kN"),
        ));
    }
    if let Some(v) = d.recycled_rubber_pct {
        elements.push(double_property("recycledRubberPct", v, None, Some("%")));
    }
    if let Some(v) = d.co2e_per_tyre_kg {
        elements.push(double_property(
            "co2ePerTyreKg",
            v,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("kgCO2e"),
        ));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:tyre-product-data"),
        id_short: "TyreProductData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::TYRE_PRODUCT_DATA)),
        submodel_elements: elements,
    }
}
