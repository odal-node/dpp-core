//! Laptops — reserved; product-specific delegated act expected ~2027.

use chrono::NaiveDate;

use super::{
    DEFAULT_REPAIRABILITY_THRESHOLDS, RepairabilityRuleset, RepairabilityThresholds,
    RepairabilityWeights,
};
use crate::ruleset::{EffectiveDateBound, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

/// EN 45554 ruleset for laptops. **Not yet in force.**
///
/// Exists as a named stub so code can reference it at compile time, gated
/// behind a `RegulatoryStatus::Provisional` check or the registry (which
/// returns `None` for dates before 2100).
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

static LAPTOP_RULESET_ID: std::sync::OnceLock<RulesetId> = std::sync::OnceLock::new();
static LAPTOP_RULESET_VERSION: std::sync::OnceLock<RulesetVersion> = std::sync::OnceLock::new();
static LAPTOP_EFFECTIVE_DATES: std::sync::OnceLock<EffectiveDateBound> = std::sync::OnceLock::new();

impl Ruleset for LaptopRuleset {
    fn id(&self) -> &RulesetId {
        LAPTOP_RULESET_ID.get_or_init(|| RulesetId("laptop-repairability".into()))
    }

    fn version(&self) -> &RulesetVersion {
        LAPTOP_RULESET_VERSION.get_or_init(|| RulesetVersion("0.0.0-stub".into()))
    }

    fn effective_dates(&self) -> &EffectiveDateBound {
        // Year 2100 sentinel: effective-date guard blocks runtime use while
        // keeping the type compile-visible for future wiring.
        LAPTOP_EFFECTIVE_DATES.get_or_init(|| {
            EffectiveDateBound::open(NaiveDate::from_ymd_opt(2100, 1, 1).expect("valid date"))
        })
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &LAPTOP_BASIS
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
