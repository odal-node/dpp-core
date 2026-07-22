//! Shared weight and threshold tables for the repairability heuristic.

use serde::{Deserialize, Serialize};

/// Weight coefficient for each heuristic parameter.
///
/// Weights must sum to 1.0. Each parameter score (0–2) is multiplied by its
/// weight and by 5.0 to produce a contribution to the 0–10 numeric score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairabilityWeights {
    pub disassembly: f64,
    pub spare_parts: f64,
    pub repair_info: f64,
    pub diagnostic_tools: f64,
    pub software_updatability: f64,
    pub customer_support: f64,
}

/// Minimum numeric score (out of 10) required for each letter grade.
///
/// Grade E is assigned when the score is below `d`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairabilityThresholds {
    /// Minimum score for grade A (highest).
    pub a: f64,
    pub b: f64,
    pub c: f64,
    /// Minimum score for grade D. Below this value → grade E.
    pub d: f64,
}

/// Default A–E band boundaries — the smartphone/tablet heuristic's own design
/// choice (see `thresholds::smartphone`), reused as a placeholder by the
/// other, not-yet-effective product categories until each gets its own band
/// boundaries from a real delegated act or a dedicated heuristic revision.
/// The four concrete rulesets share this by construction, not coincidence —
/// a single point of truth means a future change to it is a one-line edit
/// instead of a four-file find-and-replace.
pub static DEFAULT_REPAIRABILITY_THRESHOLDS: RepairabilityThresholds = RepairabilityThresholds {
    a: 8.5,
    b: 7.0,
    c: 5.5,
    d: 4.0,
};
