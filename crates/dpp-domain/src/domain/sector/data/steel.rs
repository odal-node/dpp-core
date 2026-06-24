//! Iron & Steel (EU ESPR, carbon intensity).

use serde::{Deserialize, Serialize};

use crate::domain::sector::enums::ProductionRoute;

/// Iron and Steel sector data for EU ESPR carbon intensity reporting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SteelData {
    /// 14-digit GTIN identifying the steel product.
    pub gtin: String,
    /// Carbon intensity in tonne CO₂e per tonne of steel produced.
    pub co2e_per_tonne_steel: f64,
    /// Recycled scrap content as a percentage of total input material (0.0–100.0).
    pub recycled_scrap_content_pct: f64,
    /// Steel grade / product category, e.g. `"flat"`, `"long"`, `"tube"`.
    pub product_category: String,
    /// ISO 3166-1 alpha-2 country of production.
    pub country_of_production: String,
    /// Steel production route — determines carbon intensity calculation basis.
    pub production_route: ProductionRoute,
    /// Annual production volume in tonnes (optional).
    pub annual_production_tonnes: Option<f64>,
}
