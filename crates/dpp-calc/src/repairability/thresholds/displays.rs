//! Electronic displays (TVs, monitors) — stub; ESPR delegated act expected.
//!
//! EU 2019/2021 covers ecodesign for electronic displays. An ESPR-era repairability
//! delegated act is expected. Weights below are placeholder (uniform) pending the
//! official annex. Effective-date sentinel: 2100-01-01 blocks runtime use.

use chrono::NaiveDate;

use super::{RepairabilityRuleset, RepairabilityThresholds, RepairabilityWeights};
use crate::ruleset::{EffectiveDateBound, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

pub struct DisplaysRuleset;

static DISPLAYS_WEIGHTS: RepairabilityWeights = RepairabilityWeights {
    disassembly: 0.20,
    spare_parts: 0.20,
    repair_info: 0.20,
    diagnostic_tools: 0.15,
    software_updatability: 0.15,
    customer_support: 0.10,
};

static DISPLAYS_THRESHOLDS: RepairabilityThresholds = RepairabilityThresholds {
    a: 8.5,
    b: 7.0,
    c: 5.5,
    d: 4.0,
};

static DISPLAYS_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "pending — ESPR electronic displays repairability delegated act",
    article: "TBD",
    standard: Some("EN 45554:2021"),
    technical_study: None,
    source_url: None,
    superseded_by: None,
};

static DISPLAYS_RULESET_ID: std::sync::OnceLock<RulesetId> = std::sync::OnceLock::new();
static DISPLAYS_RULESET_VERSION: std::sync::OnceLock<RulesetVersion> = std::sync::OnceLock::new();
static DISPLAYS_EFFECTIVE_DATES: std::sync::OnceLock<EffectiveDateBound> =
    std::sync::OnceLock::new();

impl Ruleset for DisplaysRuleset {
    fn id(&self) -> &RulesetId {
        DISPLAYS_RULESET_ID.get_or_init(|| RulesetId("displays-repairability".into()))
    }

    fn version(&self) -> &RulesetVersion {
        DISPLAYS_RULESET_VERSION.get_or_init(|| RulesetVersion("0.0.0-stub".into()))
    }

    fn effective_dates(&self) -> &EffectiveDateBound {
        DISPLAYS_EFFECTIVE_DATES.get_or_init(|| {
            EffectiveDateBound::open(NaiveDate::from_ymd_opt(2100, 1, 1).expect("valid date"))
        })
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
        &DISPLAYS_THRESHOLDS
    }
}
