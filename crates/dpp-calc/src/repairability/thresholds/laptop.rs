//! Laptops — reserved; product-specific delegated act expected ~2027.

use super::{
    DEFAULT_REPAIRABILITY_THRESHOLDS, RepairabilityRuleset, RepairabilityThresholds,
    RepairabilityWeights,
};
use crate::ruleset::{
    Effectivity, ParameterBasis, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion,
};

/// EN 45554 ruleset for laptops. **Not yet in force.**
///
/// Exists as a named stub so code can reference it at compile time. Its
/// effectivity is [`Effectivity::Pending`], so it governs no date at all and
/// the registry never resolves it.
pub struct LaptopRuleset;

static LAPTOP_WEIGHTS: RepairabilityWeights = RepairabilityWeights {
    disassembly: 0.20,
    spare_parts: 0.25,
    repair_info: 0.20,
    diagnostic_tools: 0.10,
    software_updatability: 0.15,
    customer_support: 0.10,
};

static LAPTOP_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "pending — ESPR laptop repairability delegated act (expected ~2027)",
    article: "TBD",
    standard: Some("EN 45554:2021"),
    technical_study: None,
    source_url: None,
    superseded_by: None,
};

static LAPTOP_RULESET_ID: RulesetId = RulesetId("laptop-repairability");
static LAPTOP_RULESET_VERSION: RulesetVersion = RulesetVersion("0.0.0-stub");
static LAPTOP_EFFECTIVITY: Effectivity = Effectivity::pending(
    "ESPR (EU) 2024/1781 — laptop ecodesign delegated act, not yet adopted",
    None,
);

impl Ruleset for LaptopRuleset {
    fn id(&self) -> &RulesetId {
        &LAPTOP_RULESET_ID
    }

    fn version(&self) -> &RulesetVersion {
        &LAPTOP_RULESET_VERSION
    }

    fn effectivity(&self) -> &Effectivity {
        &LAPTOP_EFFECTIVITY
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &LAPTOP_BASIS
    }

    /// The weights above are placeholders: no adopted act sets them, which is
    /// what [`Effectivity::Pending`] on this ruleset records.
    fn parameter_basis(&self) -> ParameterBasis {
        ParameterBasis::Assumed
    }
}

impl RepairabilityRuleset for LaptopRuleset {
    fn weights(&self) -> &RepairabilityWeights {
        &LAPTOP_WEIGHTS
    }

    fn thresholds(&self) -> &RepairabilityThresholds {
        &DEFAULT_REPAIRABILITY_THRESHOLDS
    }
}
