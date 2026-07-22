//! [`MaterialEntry`] — a single line item in the passport's bill of materials.

use serde::{Deserialize, Serialize};

/// A single material entry in the passport's bill of materials.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialEntry {
    pub name: String,
    pub weight_kg: f64,
    /// Percentage of recycled content (0.0–100.0).
    pub recycled_pct: Option<f64>,
    /// ISO 3166-1 alpha-2 country code of material origin.
    pub country_of_origin: Option<String>,
}
