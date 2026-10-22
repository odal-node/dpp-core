//! [`CriticalRawMaterial`] — a critical raw material declared in a payload.

use serde::{Deserialize, Serialize};

/// A critical raw material declared in a payload.
///
/// `name` and `cas_number` identify the material, which is what Annex VI Part A
/// point 10 of Regulation (EU) 2023/1542 makes public for a battery.
/// `weight_grams` and `country_of_origin` are the quantity and origin records of
/// Article 49(2), points (d) and (c), of the same Regulation: supply-chain due
/// diligence documentation an operator keeps for authorities and shares with its
/// immediate downstream purchasers (Article 52(1) and (2)), not passport content,
/// so a schema labels them `restricted`.
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
