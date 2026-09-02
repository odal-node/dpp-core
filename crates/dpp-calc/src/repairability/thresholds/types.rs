//! Shared weight and threshold tables for the repairability heuristic.

use serde::{Deserialize, Serialize};

use crate::parameters::RulesetParameters;

/// Name of the weights group in [`RulesetParameters`], and therefore the key a
/// bundle must use to offer replacements.
///
/// A constant rather than a literal at each site: the name a ruleset publishes
/// and the name the filled ruleset reads back have to be the same string, and
/// two literals can drift apart without anything failing to compile.
pub const WEIGHTS_GROUP: &str = "weights";

/// Name of the band-threshold group in [`RulesetParameters`]. See
/// [`WEIGHTS_GROUP`].
pub const THRESHOLDS_GROUP: &str = "thresholds";

/// Weight coefficient for each heuristic parameter.
///
/// Weights must sum to 1.0. Each parameter score (0–2) is multiplied by its
/// weight and by 5.0 to produce a contribution to the 0–10 numeric score.
///
/// `deny_unknown_fields` because these arrive from a signed bundle, where
/// serde's default — drop the unrecognised key — is wrong twice over. A key
/// added alongside the six real ones is discarded in silence and the bundle
/// reports success. A key that is a *misspelling* of one of the six is caught,
/// but only as `missing field spareParts`, which names what is absent and not
/// the `sparParts` the author actually typed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
/// Grade E is assigned when the score is below `d`. See
/// [`RepairabilityWeights`] for why unknown fields are denied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

/// The two groups every repairability ruleset declares, built from the very
/// statics it computes with.
///
/// Taking references rather than rebuilding the values is the point: a
/// `parameters()` that restated the numbers could drift from the ones the
/// calculator uses, and the receipt would then hash a set that was never
/// applied.
///
/// # Panics
///
/// Never in practice. `serde_json` rejects only non-finite floats, and every
/// weight and threshold in this crate is a finite literal; a filled set comes
/// from JSON, which has no way to express NaN or infinity.
pub(crate) fn repairability_parameters(
    weights: &RepairabilityWeights,
    thresholds: &RepairabilityThresholds,
) -> RulesetParameters {
    RulesetParameters::new()
        .with(WEIGHTS_GROUP, weights)
        .and_then(|p| p.with(THRESHOLDS_GROUP, thresholds))
        .expect("repairability weights and thresholds are finite literals")
}
