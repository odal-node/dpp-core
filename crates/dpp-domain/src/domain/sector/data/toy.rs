//! Toys (EU 2025/2509, DPP mandate 2030).

use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;

use super::shared::SvhcSubstance;

/// Toy sector data for EU Toy Safety Directive and 2025/2509 DPP compliance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToyData {
    /// 14-digit GTIN identifying the toy model.
    pub gtin: Gtin,
    /// Intended age group, e.g. `"0-3"`, `"3-6"`, `"6-12"`, `"12+"`.
    pub age_group: String,
    /// Primary material, e.g. `"plastic"`, `"wood"`, `"metal"`, `"textile"`, `"mixed"`.
    pub primary_material: String,
    /// Whether the product bears CE marking under the EU Toy Safety Directive.
    pub ce_marking: bool,
    /// ISO 3166-1 alpha-2 country of manufacture.
    pub country_of_manufacture: String,

    /// SVHC substances present above 0.1% w/w per REACH Article 33.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svhc_substances: Option<Vec<SvhcSubstance>>,
    /// Whether the toy contains a battery.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains_battery: Option<bool>,
    /// Free-text or URL pointing to repairability / spare parts information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repairability_info: Option<String>,
}
