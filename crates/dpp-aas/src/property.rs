use super::model::{AasCollection, AasDataType, AasProperty, AasSemId, AasSubmodelElement};

/// The serde wire string for `value` — factors out the
/// `serde_json::to_value(...).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()`
/// idiom repeated across the sector builders to read an enum's serde-rename
/// tag as a `String` for embedding in an AAS property.
pub fn enum_wire_str<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

/// [`enum_wire_str`] for an `Option<T>` — `None` in, `None` out; a `Some(v)`
/// that fails to serialize to a JSON string also yields `None` (matches the
/// `if let Some(v) = ... && let Some(s) = serde_json::to_value(v)...` guards
/// this replaces).
pub fn opt_enum_wire_str<T: serde::Serialize>(value: &Option<T>) -> Option<String> {
    let v = value.as_ref()?;
    serde_json::to_value(v).ok()?.as_str().map(String::from)
}

/// The `svhc_{index}` collection shape shared by every sector that reports
/// SVHC declarations: `casNumber`/`substanceName`/`concentrationPct` plus an
/// optional `locationInProduct`. Returned as a bare [`AasCollection`] (not yet
/// wrapped in [`AasSubmodelElement::SubmodelElementCollection`]) so a caller
/// needing an extra per-substance field (e.g. textile's `scipNotificationId`)
/// can push it onto `.value` before wrapping.
pub fn svhc_substance_element(
    index: usize,
    cas_number: &str,
    substance_name: &str,
    concentration_pct: f64,
    location_in_product: Option<&str>,
) -> AasCollection {
    let mut elems = vec![
        string_property("casNumber", cas_number, None, None),
        string_property("substanceName", substance_name, None, None),
        double_property("concentrationPct", concentration_pct, None, Some("%")),
    ];
    if let Some(loc) = location_in_product {
        elems.push(string_property("locationInProduct", loc, None, None));
    }
    AasCollection {
        id_short: format!("svhc_{index}"),
        value: elems,
        semantic_id: None,
    }
}

/// Create a string `Property` element.
pub fn string_property(
    id_short: &str,
    value: &str,
    semantic_id: Option<&str>,
    unit: Option<&str>,
) -> AasSubmodelElement {
    AasSubmodelElement::Property(AasProperty {
        id_short: id_short.into(),
        value_type: AasDataType::String,
        value: value.into(),
        unit: unit.map(Into::into),
        semantic_id: semantic_id.map(AasSemId::external),
        description: None,
    })
}

/// Create a double (float) `Property` element.
pub fn double_property(
    id_short: &str,
    value: f64,
    semantic_id: Option<&str>,
    unit: Option<&str>,
) -> AasSubmodelElement {
    AasSubmodelElement::Property(AasProperty {
        id_short: id_short.into(),
        value_type: AasDataType::Double,
        value: value.to_string(),
        unit: unit.map(Into::into),
        semantic_id: semantic_id.map(AasSemId::external),
        description: None,
    })
}

/// Create an integer `Property` element.
pub fn integer_property(
    id_short: &str,
    value: i64,
    semantic_id: Option<&str>,
    unit: Option<&str>,
) -> AasSubmodelElement {
    AasSubmodelElement::Property(AasProperty {
        id_short: id_short.into(),
        value_type: AasDataType::Integer,
        value: value.to_string(),
        unit: unit.map(Into::into),
        semantic_id: semantic_id.map(AasSemId::external),
        description: None,
    })
}

/// Create a boolean `Property` element.
pub fn boolean_property(
    id_short: &str,
    value: bool,
    semantic_id: Option<&str>,
    unit: Option<&str>,
) -> AasSubmodelElement {
    AasSubmodelElement::Property(AasProperty {
        id_short: id_short.into(),
        value_type: AasDataType::Boolean,
        value: value.to_string(),
        unit: unit.map(Into::into),
        semantic_id: semantic_id.map(AasSemId::external),
        description: None,
    })
}
