//! Detergents & Surfactants (EU 2026/405, mandate 2029).

use serde::{Deserialize, Serialize};

/// A single surfactant ingredient in a detergent product.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SurfactantEntry {
    /// INCI or IUPAC name of the surfactant.
    pub name: String,
    /// Whether the surfactant is readily biodegradable per EU Regulation 2026/405.
    /// All surfactants must be readily biodegradable; no derogations apply.
    pub biodegradable: bool,
    /// Concentration band per EU detergent labelling convention (Annex VII of 2026/405):
    /// `"<5%"`, `"5-15%"`, `"15-30%"`, or `">=30%"` weight/weight.
    pub concentration_band: String,
    /// CAS Registry Number if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cas_number: Option<String>,
}

/// Detergent and surfactant sector data for EU Regulation 2026/405 DPP compliance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DetergentData {
    /// 14-digit GTIN identifying the detergent product.
    pub gtin: String,
    /// Product type, e.g. `"laundry"`, `"dishwashing"`, `"surface-cleaner"`, `"personal-care"`, `"other"`.
    pub product_type: String,
    /// Physical format, e.g. `"liquid"`, `"powder"`, `"tablet"`, `"gel"`, `"concentrate"`.
    pub format: String,
    /// List of surfactants in the product formulation.
    pub surfactants: Vec<SurfactantEntry>,
    /// ISO 3166-1 alpha-2 country of manufacture.
    pub country_of_manufacture: String,

    /// Carbon footprint in kg CO₂e per product unit (bottle or pack).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co2e_per_unit_kg: Option<f64>,
    /// Whether the primary packaging is recyclable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packaging_recyclable: Option<bool>,
    /// Recommended dosage in millilitres (or grams for powder).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_dosage_ml: Option<f64>,
    /// Whether all surfactants are readily biodegradable (convenience summary flag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub biodegradable: Option<bool>,
}
