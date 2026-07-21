//! Construction Products (EU CPR 2024/3110, mandate ~2028-2032 phased).

use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;

/// Construction products sector data for EU CPR 2024/3110 compliance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConstructionData {
    /// 14-digit GTIN identifying the construction product.
    pub gtin: Gtin,
    /// Product family, e.g. `"cement"`, `"concrete"`, `"structural-steel"`, `"glass"`.
    pub product_family: String,
    /// ISO 3166-1 alpha-2 country of manufacture.
    pub country_of_manufacture: String,
    /// Carbon footprint in kg CO₂e per functional unit.
    pub co2e_per_functional_unit_kg: f64,
    /// Description of the functional unit (e.g., `"per tonne"`, `"per m²"`, `"per m³"`).
    pub functional_unit: String,

    /// Recycled content as a percentage of total input material (0.0–100.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_pct: Option<f64>,
    /// URL to the Environmental Product Declaration (EPD).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epd_url: Option<String>,
    /// Whether the product carries CE marking under EU CPR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ce_marking: Option<bool>,
}
