//! Electronic displays (TVs, monitors) — stub; ESPR delegated act expected.
//!
//! EU 2019/2021 covers ecodesign for electronic displays. An ESPR-era repairability
//! delegated act is expected. Weights below are placeholder (uniform) pending the
//! official annex. Effectivity is Pending, so this ruleset governs no date.

use super::{
    DEFAULT_REPAIRABILITY_THRESHOLDS, RepairabilityRuleset, RepairabilityThresholds,
    RepairabilityWeights,
};
use crate::ruleset::{Effectivity, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

pub struct DisplaysRuleset;

static DISPLAYS_WEIGHTS: RepairabilityWeights = RepairabilityWeights {
    disassembly: 0.20,
    spare_parts: 0.20,
    repair_info: 0.20,
    diagnostic_tools: 0.15,
    software_updatability: 0.15,
    customer_support: 0.10,
};

static DISPLAYS_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "pending — ESPR electronic displays repairability delegated act",
    article: "TBD",
    standard: Some("EN 45554:2021"),
    technical_study: None,
    source_url: None,
    superseded_by: None,
};

static DISPLAYS_RULESET_ID: RulesetId = RulesetId("displays-repairability");
static DISPLAYS_RULESET_VERSION: RulesetVersion = RulesetVersion("0.0.0-stub");
static DISPLAYS_EFFECTIVITY: Effectivity = Effectivity::pending(
    "ESPR (EU) 2024/1781 — electronic displays repairability delegated act, not yet adopted",
    None,
);

impl Ruleset for DisplaysRuleset {
    fn id(&self) -> &RulesetId {
        &DISPLAYS_RULESET_ID
    }

    fn version(&self) -> &RulesetVersion {
        &DISPLAYS_RULESET_VERSION
    }

    fn effectivity(&self) -> &Effectivity {
        &DISPLAYS_EFFECTIVITY
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &DISPLAYS_BASIS
    }
}

impl RepairabilityRuleset for DisplaysRuleset {
    fn weights(&self) -> &RepairabilityWeights {
        &DISPLAYS_WEIGHTS
    }

    fn thresholds(&self) -> &RepairabilityThresholds {
        &DEFAULT_REPAIRABILITY_THRESHOLDS
    }
}
