//! [`CarbonFootprint`] — a provenance-aware, methodology-tagged CO₂e declaration.

use serde::{Deserialize, Serialize};

use super::{CarbonFootprintClass, LifecycleStage, SystemBoundary};

/// Structured carbon footprint declaration — replaces bare `co2e_per_unit: f64`
/// on [`Passport`](crate::passport::Passport) with provenance-aware,
/// methodology-tagged data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarbonFootprint {
    /// CO₂-equivalent value in kg per functional unit.
    pub value_kg: f64,
    /// LCA lifecycle stage covered by this figure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle_stage: Option<LifecycleStage>,
    /// LCA system-boundary standard used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_boundary: Option<SystemBoundary>,
    /// Reference to the methodology document (URL or standard identifier).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub methodology_ref: Option<String>,
    /// Performance class label assigned by the manufacturer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance_class: Option<CarbonFootprintClass>,
}

impl CarbonFootprint {
    /// Construct a minimal footprint from a scalar kg CO₂e value.
    pub fn from_kg(value_kg: f64) -> Self {
        Self {
            value_kg,
            lifecycle_stage: None,
            system_boundary: None,
            methodology_ref: None,
            performance_class: None,
        }
    }
}
