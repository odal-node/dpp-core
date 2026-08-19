//! The Art. 8 determination: declared shares → shortfalls, with a receipt.

use serde::{Deserialize, Serialize};

use super::parameters::RecycledContentInputs;
use super::thresholds::RecycledContentRuleset;
use crate::clock::AssessmentClock;
use crate::error::CalcError;
use crate::receipt::{CalculationReceipt, jcs_hash};

/// One metal whose declared share is below the governing minimum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetalShortfall {
    /// `"cobalt"`, `"lithium"`, `"nickel"` or `"lead"`.
    pub metal: String,
    /// The share the operator declared, percent.
    pub declared_pct: f64,
    /// The minimum the governing phase requires, percent.
    pub required_pct: f64,
}

/// The determination, plus the receipt that makes it re-checkable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecycledContentResult {
    /// Metals below the governing minimum. Empty means every **declared** share
    /// meets it — not that every share was declared. An undeclared share is a
    /// missing declaration, which is a different finding and belongs to the
    /// Art. 8(1) duty, not to the minimum-share determination.
    pub shortfalls: Vec<MetalShortfall>,
    /// Proof of calculation: the input hash, this ruleset's id and version, and
    /// the date whose law was applied.
    pub receipt: CalculationReceipt,
}

/// Determine which declared recycled-content shares fall below the minimums the
/// law in force on `clock.law_in_force_on` sets for this battery.
///
/// `ruleset` is the phase that governs — resolve it with
/// [`resolve_recycled_content`](crate::ruleset_registry::resolve_recycled_content)
/// rather than choosing one, because Art. 8(2) and Art. 8(3) differ in *scope*
/// as well as date: Art. 8(2) does not reach LMT batteries at all.
///
/// # Errors
///
/// - [`CalcError::InvalidInput`] for a share that is not a finite percentage.
///   A share outside 0–100 is not a conservative reading of a bad declaration,
///   it is an uninterpretable one.
/// - [`CalcError::RulesetNotYetEffective`] / [`CalcError::RulesetExpired`] when
///   the phase does not govern a battery placed on the market on that date.
///   Being outside a phase is **not** a shortfall, and reporting it as one would
///   assert an obligation the battery does not carry.
pub fn calculate(
    inputs: &RecycledContentInputs,
    ruleset: &dyn RecycledContentRuleset,
    clock: AssessmentClock,
) -> Result<RecycledContentResult, CalcError> {
    validate_inputs(inputs)?;
    ruleset
        .effectivity()
        .ensure_active_on(ruleset.id(), clock.law_in_force_on)?;

    let shortfalls = ruleset.shortfalls(inputs);

    let output_hash = jcs_hash(&shortfalls)?;
    let receipt = CalculationReceipt::for_ruleset(inputs, ruleset, clock, output_hash)?;

    Ok(RecycledContentResult {
        shortfalls,
        receipt,
    })
}

fn validate_inputs(inputs: &RecycledContentInputs) -> Result<(), CalcError> {
    for (name, share) in [
        ("cobalt_pct", inputs.cobalt_pct),
        ("lithium_pct", inputs.lithium_pct),
        ("nickel_pct", inputs.nickel_pct),
        ("lead_pct", inputs.lead_pct),
    ] {
        if let Some(v) = share
            && (!v.is_finite() || !(0.0..=100.0).contains(&v))
        {
            return Err(CalcError::InvalidInput(format!(
                "{name} must be a finite percentage in 0..=100; got {v}"
            )));
        }
    }
    Ok(())
}
