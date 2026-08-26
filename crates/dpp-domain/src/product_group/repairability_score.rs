//! [`RepairabilityScore`] — an overall score and the criteria behind it.
//!
//! A non-regulatory heuristic. Not EN 45554, and not Regulation (EU) 2023/1669.

use serde::{Deserialize, Serialize};

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
/// on [`Passport`](crate::passport::Passport) with a breakdown by criterion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairabilityScore {
    /// Overall score 0.0–10.0. Non-regulatory heuristic — not EN 45554 / EU 2023/1669.
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
