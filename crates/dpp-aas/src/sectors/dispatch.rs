//! Sector-data dispatch: routes a [`SectorData`] variant to its AAS submodel builder.

use dpp_domain::SectorData;

use crate::mapper::json_value_to_element;
use crate::model::AasSubmodel;

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
        SectorData::UnsoldGoods(r) => {
            super::unsold_goods::build_unsold_goods_submodel(r, passport_id)
        }
        SectorData::Other { data: v, .. } => generic_sector_submodel(passport_id, v),
        // Every sector without a typed mapper — which is every sector whose
        // act is not in force — renders as a generic submodel from its
        // serialised fields, the same shape as `Other`. A generic projection is
        // the honest one for a sector whose ratified template does not exist.
        other => {
            let value = serde_json::to_value(other).unwrap_or(serde_json::Value::Null);
            generic_sector_submodel(passport_id, &value)
        }
    }
}
