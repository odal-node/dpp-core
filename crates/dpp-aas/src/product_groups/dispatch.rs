//! ProductGroup-data dispatch: routes a [`ProductGroupData`] variant to its AAS submodel builder.

use dpp_domain::ProductGroupData;

use crate::mapper::json_value_to_element;
use crate::model::AasSubmodel;

/// Build a generic "unmodelled product group" submodel from a JSON object's fields
/// (minus the `product_group` discriminant key) — the fallback shape shared by both
/// `ProductGroupData::Other` and any product group variant this crate doesn't yet have a
/// dedicated builder for.
fn generic_product_group_submodel(passport_id: &str, value: &serde_json::Value) -> AasSubmodel {
    let elements = match value {
        serde_json::Value::Object(map) => map
            .iter()
            .filter(|(k, _)| k.as_str() != "productGroup")
            .map(|(k, v)| json_value_to_element(k, v))
            .collect(),
        _ => vec![],
    };
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:product_group-data"),
        id_short: "ProductGroupData".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: None,
        submodel_elements: elements,
    }
}

pub(crate) fn build_product_group_submodel(
    product_group_data: &ProductGroupData,
    passport_id: &str,
) -> AasSubmodel {
    match product_group_data {
        ProductGroupData::Battery(b) => super::battery::build_battery_submodel(b, passport_id),
        ProductGroupData::Textile(t) => super::textile::build_textile_submodel(t, passport_id),
        ProductGroupData::Electronics(e) => {
            super::electronics::build_electronics_submodel(e, passport_id)
        }
        ProductGroupData::UnsoldGoods(r) => {
            super::unsold_goods::build_unsold_goods_submodel(r, passport_id)
        }
        ProductGroupData::Other { data: v, .. } => generic_product_group_submodel(passport_id, v),
        // Every product group without a typed mapper — which is every product group whose
        // act is not in force — renders as a generic submodel from its
        // serialised fields, the same shape as `Other`. A generic projection is
        // the honest one for a product group whose ratified template does not exist.
        other => {
            let value = serde_json::to_value(other).unwrap_or(serde_json::Value::Null);
            generic_product_group_submodel(passport_id, &value)
        }
    }
}
