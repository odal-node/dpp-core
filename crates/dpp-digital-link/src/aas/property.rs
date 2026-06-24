use super::model::{AasDataType, AasProperty, AasSemId, AasSubmodelElement};

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
