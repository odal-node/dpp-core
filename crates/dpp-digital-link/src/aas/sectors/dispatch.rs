//! Sector-data dispatch: routes a [`SectorData`] variant to its AAS submodel builder.

use dpp_domain::SectorData;

use crate::aas::mapper::json_value_to_element;
use crate::aas::model::AasSubmodel;

/// Build a generic "unmodelled sector" submodel from a JSON object's fields
/// (minus the `sector` discriminant key) — the fallback shape shared by both
/// `SectorData::Other` and any sector variant this crate doesn't yet have a
/// dedicated builder for.
fn generic_sector_submodel(passport_id: &str, value: &serde_json::Value) -> AasSubmodel {
    let elements = match value {
        serde_json::Value::Object(map) => map
            .iter()
            .filter(|(k, _)| k.as_str() != "sector")
            .map(|(k, v)| json_value_to_element(k, v))
            .collect(),
        _ => vec![],
    };
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:sector-data"),
        id_short: "SectorData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: None,
        submodel_elements: elements,
    }
}

pub(crate) fn build_sector_submodel(sector_data: &SectorData, passport_id: &str) -> AasSubmodel {
    match sector_data {
        SectorData::Battery(b) => super::battery::build_battery_submodel(b, passport_id),
        SectorData::Textile(t) => super::textile::build_textile_submodel(t, passport_id),
        SectorData::Electronics(e) => {
            super::electronics::build_electronics_submodel(e, passport_id)
        }
        SectorData::Steel(d) => super::steel::build_steel_submodel(d, passport_id),
        SectorData::Construction(d) => {
            super::construction::build_construction_submodel(d, passport_id)
        }
        SectorData::Tyre(d) => super::tyre::build_tyre_submodel(d, passport_id),
        SectorData::Toy(d) => super::toy::build_toy_submodel(d, passport_id),
        SectorData::Aluminium(d) => super::aluminium::build_aluminium_submodel(d, passport_id),
        SectorData::Furniture(d) => super::furniture::build_furniture_submodel(d, passport_id),
        SectorData::Detergent(d) => super::detergent::build_detergent_submodel(d, passport_id),
        SectorData::UnsoldGoods(r) => {
            super::unsold_goods::build_unsold_goods_submodel(r, passport_id)
        }
        SectorData::Other(v) => generic_sector_submodel(passport_id, v),
        // Forward-compat: an unmodelled sector variant is rendered as a generic
        // submodel from its serialised fields (same shape as `Other`).
        other => {
            let value = serde_json::to_value(other).unwrap_or(serde_json::Value::Null);
            generic_sector_submodel(passport_id, &value)
        }
    }
}
