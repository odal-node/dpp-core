use super::model::{AasCollection, AasSemId, AasSubmodel, AasSubmodelElement};
use super::property::{boolean_property, double_property, integer_property, string_property};

/// Map a raw JSON payload to a single AAS Submodel.
///
/// This is a generic escape hatch. For typed passports use
/// [`build_aas_from_passport`](super::build_aas_from_passport), which produces
/// multiple named submodels with
/// semantic IDs and physical units.
///
/// `submodel_id` should be a unique URI for this DPP instance.
pub fn map_dpp_to_aas_submodel(submodel_id: &str, dpp_data: &serde_json::Value) -> AasSubmodel {
    let elements = match dpp_data {
        serde_json::Value::Object(map) => map
            .iter()
            .map(|(key, val)| json_value_to_element(key, val))
            .collect(),
        _ => vec![],
    };
    AasSubmodel {
        id: submodel_id.into(),
        id_short: "DigitalProductPassport".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(
            "urn:idta:aas:submodel:digital-product-passport:1.0",
        )),
        submodel_elements: elements,
    }
}

pub(crate) fn json_value_to_element(key: &str, value: &serde_json::Value) -> AasSubmodelElement {
    match value {
        serde_json::Value::String(s) => string_property(key, s, None, None),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                integer_property(key, i, None, None)
            } else if let Some(f) = n.as_f64() {
                double_property(key, f, None, None)
            } else {
                string_property(key, &n.to_string(), None, None)
            }
        }
        serde_json::Value::Bool(b) => boolean_property(key, *b, None, None),
        serde_json::Value::Object(map) => {
            let children: Vec<AasSubmodelElement> = map
                .iter()
                .map(|(k, v)| json_value_to_element(k, v))
                .collect();
            AasSubmodelElement::SubmodelElementCollection(AasCollection {
                id_short: key.into(),
                value: children,
                semantic_id: None,
            })
        }
        serde_json::Value::Array(arr) => {
            let children: Vec<AasSubmodelElement> = arr
                .iter()
                .enumerate()
                .map(|(i, v)| json_value_to_element(&format!("item_{i}"), v))
                .collect();
            AasSubmodelElement::SubmodelElementCollection(AasCollection {
                id_short: key.into(),
                value: children,
                semantic_id: None,
            })
        }
        serde_json::Value::Null => string_property(key, "", None, None),
    }
}
