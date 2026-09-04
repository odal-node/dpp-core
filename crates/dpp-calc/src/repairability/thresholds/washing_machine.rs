//! Washing machines / washer-dryers — stub; ESPR delegated act expected.
//!
//! EU 2021/341 includes repairability requirements for washing machines. An
//! ESPR-era repairability index delegated act is expected ~2026. Weights and
//! thresholds below are placeholder pending the official annex.

use super::{
    DEFAULT_REPAIRABILITY_THRESHOLDS, RepairabilityRuleset, RepairabilityThresholds,
    RepairabilityWeights,
};
use crate::ruleset::{
    Effectivity, ParameterBasis, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion,
};

pub struct WashingMachineRuleset;

static WASHING_WEIGHTS: RepairabilityWeights = RepairabilityWeights {
    disassembly: 0.25,
    spare_parts: 0.20,
    repair_info: 0.20,
    diagnostic_tools: 0.15,
    software_updatability: 0.10,
    customer_support: 0.10,
};

static WASHING_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "pending — ESPR washing machine repairability delegated act (expected ~2026)",
    article: "TBD",
    standard: Some("EN 45554:2021"),
    technical_study: None,
    source_url: None,
    superseded_by: None,
};

static WASHING_RULESET_ID: RulesetId = RulesetId("washing-machine-repairability");
static WASHING_RULESET_VERSION: RulesetVersion = RulesetVersion("0.0.0-stub");
static WASHING_EFFECTIVITY: Effectivity = Effectivity::pending(
    "ESPR (EU) 2024/1781 — washing machine repairability delegated act, not yet adopted",
    None,
);

impl Ruleset for WashingMachineRuleset {
    fn id(&self) -> &RulesetId {
        &WASHING_RULESET_ID
    }

    fn version(&self) -> &RulesetVersion {
        &WASHING_RULESET_VERSION
    }

    fn effectivity(&self) -> &Effectivity {
        &WASHING_EFFECTIVITY
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &WASHING_BASIS
    }

    /// The weights above are placeholders: no adopted act sets them, which is
    /// what [`Effectivity::Pending`] on this ruleset records.
    fn parameter_basis(&self) -> ParameterBasis {
        ParameterBasis::Assumed
    }
}

impl RepairabilityRuleset for WashingMachineRuleset {
    fn weights(&self) -> &RepairabilityWeights {
        &WASHING_WEIGHTS
    }

    fn thresholds(&self) -> &RepairabilityThresholds {
        &DEFAULT_REPAIRABILITY_THRESHOLDS
    }
}
