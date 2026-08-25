//! [`CriticalRawMaterial`] — a critical raw material declared in a payload.

use serde::{Deserialize, Serialize};

/// Critical raw material declaration per EU CRM Act 2024/1252.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CriticalRawMaterial {
    /// Material name, e.g. `"cobalt"`, `"lithium"`, `"natural graphite"`.
    pub name: String,
    /// CAS or EC number for unambiguous identification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cas_number: Option<String>,
    /// Weight in grams present in the battery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_grams: Option<f64>,
    /// ISO 3166-1 alpha-2 country of primary extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_of_origin: Option<String>,
}
