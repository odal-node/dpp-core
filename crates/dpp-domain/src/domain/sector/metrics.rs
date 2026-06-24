//! Structured environmental metrics (3.2a).
//!
//! Provenance-aware carbon footprint and repairability scoring types shared by
//! [`Passport`](crate::domain::passport::Passport) and several sector structs.

use serde::{Deserialize, Serialize};

use crate::domain::sector::enums::{CarbonFootprintClass, LifecycleStage, SystemBoundary};

/// Structured carbon footprint declaration — replaces bare `co2e_per_unit: f64`
/// on [`Passport`](crate::domain::passport::Passport) with provenance-aware,
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

/// A single criterion contributing to a product's repairability score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairCriterion {
    /// Criterion name, e.g. `"spare-parts-availability"`, `"ease-of-disassembly"`.
    pub name: String,
    /// Score for this criterion (same scale as the overall score).
    pub score: f64,
    /// Relative weight of this criterion in the overall score calculation.
    pub weight: f64,
}

/// Structured repairability score — replaces bare `repairability_score: f64`
/// on [`Passport`](crate::domain::passport::Passport) with a breakdown by criterion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairabilityScore {
    /// Overall score 0.0–10.0 per EU ecodesign scoring methodology.
    pub overall: f64,
    /// Breakdown by individual criterion (may be empty if only overall is known).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub criteria: Vec<RepairCriterion>,
}

impl RepairabilityScore {
    /// Construct a score from a bare overall scalar (no criterion breakdown).
    pub fn from_scalar(overall: f64) -> Self {
        Self {
            overall,
            criteria: Vec::new(),
        }
    }
}
