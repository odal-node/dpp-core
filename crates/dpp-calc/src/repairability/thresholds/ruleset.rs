//! The [`RepairabilityRuleset`] trait implemented by each concrete ruleset version.

use super::{RepairabilityThresholds, RepairabilityWeights};
use crate::error::CalcError;
use crate::repairability::parameters::RepairabilityInputs;
use crate::ruleset::Ruleset;

/// Ruleset for the simplified repairability heuristic.
///
/// Extends [`Ruleset`]; `regulatory_basis()` records the provenance, which for
/// the heuristic is explicitly non-regulatory. The heuristic-specific data
/// (weights and A–E band thresholds) is in the two additional methods.
///
/// ## Canonical pattern for future methodology traits
/// ```rust,ignore
/// pub trait PEFRuleset: Ruleset { ... }
/// pub trait CfbRuleset: Ruleset { ... }
/// ```
pub trait RepairabilityRuleset: Ruleset {
    fn weights(&self) -> &RepairabilityWeights;
    fn thresholds(&self) -> &RepairabilityThresholds;

    /// Validate parameter combinations that are incoherent per this ruleset.
    ///
    /// Called after range validation. Default: no cross-field constraints.
    /// Override to enforce coherence rules (e.g. the disassembly/spare-parts
    /// dependency).
    fn validate_cross_fields(&self, _inputs: &RepairabilityInputs) -> Result<(), CalcError> {
        Ok(())
    }
}
