//! Aluminium (EU ESPR ~2030, CBAM-aligned).

use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;
use crate::domain::sector::enums::ProductionRoute;

/// Aluminium sector data for EU ESPR carbon intensity reporting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AluminiumData {
    /// 14-digit GTIN identifying the aluminium product.
    pub gtin: Gtin,
    /// Alloy series designation, e.g. `"1xxx"`, `"3xxx"`, `"5xxx"`, `"6xxx"`.
    pub alloy_grade: String,
    /// Aluminium production route — determines carbon intensity calculation basis.
    pub production_route: ProductionRoute,
    /// Carbon intensity in kg CO₂e per tonne of aluminium produced.
    pub co2e_per_tonne_kg: f64,
    /// Recycled scrap content as a percentage of total input (0.0–100.0).
    pub recycled_content_pct: f64,
    /// ISO 3166-1 alpha-2 country of primary production.
    pub country_of_production: String,
    /// Annual production volume in tonnes (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annual_production_tonnes: Option<f64>,
}
