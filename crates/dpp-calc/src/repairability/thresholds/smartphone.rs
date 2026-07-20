//! Simplified repairability heuristic for smartphones/tablets (applied today).
//!
//! NON-REGULATORY. This is a transparent six-factor 0–2 indicator, NOT the
//! enacted EU 2023/1669 Annex IV repairability index (which uses a different
//! parameter set incl. Fasteners & Tools, a 1–5 per-class scale, a priority-part
//! dimension, and its own class boundaries — see docs/audit H-1). The weights and
//! band thresholds below are heuristic design choices; they are deliberately
//! NOT pinned to any OJ annex, and the output must not be presented as a
//! regulatory repairability class.

use chrono::NaiveDate;

use super::{
    DEFAULT_REPAIRABILITY_THRESHOLDS, RepairabilityRuleset, RepairabilityThresholds,
    RepairabilityWeights,
};
use crate::error::CalcError;
use crate::repairability::parameters::RepairabilityInputs;
use crate::ruleset::{EffectiveDateBound, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

static SMARTPHONE_WEIGHTS: RepairabilityWeights = RepairabilityWeights {
    disassembly: 0.25,
    spare_parts: 0.15,
    repair_info: 0.15,
    diagnostic_tools: 0.15,
    software_updatability: 0.15,
    customer_support: 0.15,
};

static SMARTPHONE_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "Non-regulatory: simplified repairability heuristic (NOT EU 2023/1669 Annex IV)",
    article: "Six-factor heuristic — disassembly, spare parts, repair info, \
              diagnostic tools, software updates, customer support",
    standard: None,
    technical_study: None,
    source_url: None,
    superseded_by: None,
};

static SMARTPHONE_RULESET_ID: std::sync::OnceLock<RulesetId> = std::sync::OnceLock::new();
static SMARTPHONE_RULESET_VERSION: std::sync::OnceLock<RulesetVersion> = std::sync::OnceLock::new();
static SMARTPHONE_EFFECTIVE_DATES: std::sync::OnceLock<EffectiveDateBound> =
    std::sync::OnceLock::new();

/// Simplified, non-regulatory repairability heuristic — a transparent six-factor
/// 0–2 indicator, applied to smartphones/tablets today. **Not** the enacted EU
/// 2023/1669 Annex IV index; the output is a heuristic band, not a regulatory
/// class. Available from 2025-06-20 (when the heuristic was introduced).
pub struct SimplifiedRepairabilityHeuristic;

impl Ruleset for SimplifiedRepairabilityHeuristic {
    fn id(&self) -> &RulesetId {
        SMARTPHONE_RULESET_ID.get_or_init(|| RulesetId("repairability-heuristic-v1".into()))
    }

    fn version(&self) -> &RulesetVersion {
        SMARTPHONE_RULESET_VERSION.get_or_init(|| RulesetVersion("1.0.0".into()))
    }

    fn effective_dates(&self) -> &EffectiveDateBound {
        SMARTPHONE_EFFECTIVE_DATES.get_or_init(|| {
            EffectiveDateBound::open(NaiveDate::from_ymd_opt(2025, 6, 20).expect("valid date"))
        })
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &SMARTPHONE_BASIS
    }
}

impl RepairabilityRuleset for SimplifiedRepairabilityHeuristic {
    fn weights(&self) -> &RepairabilityWeights {
        &SMARTPHONE_WEIGHTS
    }

    fn thresholds(&self) -> &RepairabilityThresholds {
        &DEFAULT_REPAIRABILITY_THRESHOLDS
    }

    fn validate_cross_fields(&self, inputs: &RepairabilityInputs) -> Result<(), CalcError> {
        // Coherence rule: spare-parts availability presupposes disassembly.
        // A score of 0 for disassembly combined with any spare-parts score > 0 is
        // incoherent — the product cannot be repaired if it cannot be opened.
        if inputs.disassembly == 0 && inputs.spare_parts > 0 {
            return Err(CalcError::CrossFieldViolation(
                "spare_parts requires disassembly ≥ 1: parts are inaccessible \
                 without disassembly instructions"
                    .into(),
            ));
        }
        Ok(())
    }
}
