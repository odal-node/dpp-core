//! A repairability ruleset whose parameters a signed bundle has filled.
//!
//! The first real consumer of the ruleset-bundle format. Until now the channel
//! verified payloads that nothing read, so its fail-closed path had never been
//! exercised from a caller whose behaviour depended on it.

use dpp_rules::bundle::RulesetAcceptance;

use super::{
    RepairabilityRuleset, RepairabilityThresholds, RepairabilityWeights, THRESHOLDS_GROUP,
    WEIGHTS_GROUP,
};
use crate::error::CalcError;
use crate::parameters::{BundleProvenance, RulesetParameters, fill, offered_for};
use crate::repairability::parameters::RepairabilityInputs;
use crate::ruleset::{
    Effectivity, ParameterBasis, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion,
};

/// Tolerance on the weight-sum check.
///
/// The weights are decimal literals in the bundle and in this crate, and the
/// nearest `f64` to a sum of six of them is not exactly 1.0. Wide enough for
/// that, far too narrow to admit a set that is actually wrong.
const WEIGHT_SUM_TOLERANCE: f64 = 1e-9;

/// A repairability ruleset carrying parameters from a verified bundle.
///
/// Identity is the base ruleset's, unchanged: a bundle delivers numbers, never
/// a ruleset, so `id()` and `version()` still come from the compiled-in literal.
/// What differs is [`weights`](RepairabilityRuleset::weights),
/// [`thresholds`](RepairabilityRuleset::thresholds), and the provenance a
/// receipt will carry.
///
/// Implements [`RepairabilityRuleset`], so
/// [`calculate`](crate::repairability::calculate) takes one without knowing a
/// bundle exists.
pub struct FilledRepairabilityRuleset<'a> {
    base: &'a dyn RepairabilityRuleset,
    weights: RepairabilityWeights,
    thresholds: RepairabilityThresholds,
    parameters: RulesetParameters,
    provenance: BundleProvenance,
}

impl<'a> FilledRepairabilityRuleset<'a> {
    /// Adopt the parameters `acceptance` offers for `base`, or `Ok(None)` if it
    /// offers none.
    ///
    /// `Ok(None)` is the ordinary case — a bundle carries slices for the
    /// rulesets it means to change and says nothing about the rest, and the
    /// caller stays on its compiled-in numbers. It is not an error and should
    /// not be logged as one.
    ///
    /// # Errors
    ///
    /// - The acceptance is not [`Verified`](dpp_rules::bundle::RulesetProvenance::Verified).
    /// - `base` is [`ParameterBasis::Sourced`] — its numbers are the
    ///   instrument's and a bundle may not override them.
    /// - The offered slice names a group `base` does not declare, or changes a
    ///   group's JSON type.
    /// - The filled weights do not sum to 1.0, or the filled band thresholds are
    ///   not strictly descending.
    ///
    /// # Why the last two are checked here and not in the kernel
    ///
    /// [`fill`] enforces provenance, which is the same question for every
    /// methodology. Whether six weights sum to one is a fact about *this*
    /// methodology, and the kernel has no business knowing it. Something has to
    /// check: nothing else does, and a bundle whose weights sum to 1.6 would
    /// otherwise produce a "0–10" score of 16 and a band of A, with a receipt
    /// attesting to all of it.
    pub fn adopt(
        base: &'a dyn RepairabilityRuleset,
        acceptance: &RulesetAcceptance,
    ) -> Result<Option<Self>, CalcError> {
        let Some((offered, provenance)) = offered_for(acceptance, base.id())? else {
            return Ok(None);
        };

        let parameters = fill(base, &offered)?;
        let weights: RepairabilityWeights = parameters.typed_group(WEIGHTS_GROUP)?;
        let thresholds: RepairabilityThresholds = parameters.typed_group(THRESHOLDS_GROUP)?;

        validate_weights(base.id(), &weights)?;
        validate_thresholds(base.id(), &thresholds)?;

        Ok(Some(Self {
            base,
            weights,
            thresholds,
            parameters,
            provenance,
        }))
    }
}

/// The six weights must sum to 1.0, or the 0–10 score is not on a 0–10 scale.
fn validate_weights(id: &RulesetId, w: &RepairabilityWeights) -> Result<(), CalcError> {
    let sum = w.disassembly
        + w.spare_parts
        + w.repair_info
        + w.diagnostic_tools
        + w.software_updatability
        + w.customer_support;

    if !sum.is_finite() || (sum - 1.0).abs() > WEIGHT_SUM_TOLERANCE {
        return Err(CalcError::InvalidInput(format!(
            "ruleset '{}': bundle weights sum to {sum}, not 1.0 — the score would not be on \
             the 0–10 scale the band thresholds are written against",
            id.0
        )));
    }
    Ok(())
}

/// Bands must be strictly descending, or a score can qualify for two of them
/// and the first branch taken silently wins.
fn validate_thresholds(id: &RulesetId, t: &RepairabilityThresholds) -> Result<(), CalcError> {
    if !(t.a > t.b && t.b > t.c && t.c > t.d) {
        return Err(CalcError::InvalidInput(format!(
            "ruleset '{}': bundle band thresholds must be strictly descending, got \
             a={}, b={}, c={}, d={}",
            id.0, t.a, t.b, t.c, t.d
        )));
    }
    Ok(())
}

/// Hand-written because `base` is a trait object with no `Debug` bound, and
/// adding one would constrain every `RepairabilityRuleset` implementor for the
/// sake of this wrapper. The base's identity is what a reader wants anyway.
impl std::fmt::Debug for FilledRepairabilityRuleset<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilledRepairabilityRuleset")
            .field("ruleset_id", &self.base.id().0)
            .field("weights", &self.weights)
            .field("thresholds", &self.thresholds)
            .field("provenance", &self.provenance)
            .finish()
    }
}

impl Ruleset for FilledRepairabilityRuleset<'_> {
    fn id(&self) -> &RulesetId {
        self.base.id()
    }

    fn version(&self) -> &RulesetVersion {
        self.base.version()
    }

    fn effectivity(&self) -> &Effectivity {
        self.base.effectivity()
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        self.base.regulatory_basis()
    }

    /// Still [`ParameterBasis::Assumed`] — filling does not make numbers law.
    ///
    /// `adopt` only accepts an `Assumed` base, so this is that basis carried
    /// through rather than a fresh claim. It matters that it is not upgraded: a
    /// bundle that could make its own figures read as `Sourced` would let a
    /// second bundle be refused on the strength of the first one's say-so, and
    /// for the heuristic in particular the numbers stay openly non-regulatory
    /// however they arrived.
    fn parameter_basis(&self) -> ParameterBasis {
        self.base.parameter_basis()
    }

    fn parameters(&self) -> RulesetParameters {
        self.parameters.clone()
    }

    fn bundle_provenance(&self) -> Option<&BundleProvenance> {
        Some(&self.provenance)
    }
}

impl RepairabilityRuleset for FilledRepairabilityRuleset<'_> {
    fn weights(&self) -> &RepairabilityWeights {
        &self.weights
    }

    fn thresholds(&self) -> &RepairabilityThresholds {
        &self.thresholds
    }

    /// Delegated: cross-field coherence is a property of the methodology, not of
    /// the numbers, so a bundle does not get to relax it.
    fn validate_cross_fields(&self, inputs: &RepairabilityInputs) -> Result<(), CalcError> {
        self.base.validate_cross_fields(inputs)
    }
}
