//! Ruleset trait and concrete cradle-to-gate ruleset for the CO₂e methodology.

use chrono::NaiveDate;
use std::sync::OnceLock;

use crate::ruleset::{EffectiveDateBound, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

use super::LifecycleStage;

/// Regulatory ruleset for a CO₂e methodology.
///
/// Extends [`Ruleset`] — every implementation must provide a legal citation
/// via `regulatory_basis()`. The methodology-specific data (lifecycle stages
/// in scope, allocation rules) is added via additional methods.
pub trait Co2eRuleset: Ruleset {
    /// Lifecycle stages this ruleset covers (the declared PEF system boundary).
    fn declared_stages(&self) -> &[LifecycleStage];
}

static CTG_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "EU PEF Method v3.1 (cradle-to-gate production stage)",
    article: "Section 9 (system boundary), Table 5 (impact categories)",
    standard: None,
    technical_study: Some("JRC Technical Report EUR 31154 EN (PEF Method v3.1, 2021)"),
    source_url: Some("https://eplca.jrc.ec.europa.eu/EnvironmentalFootprint.html"),
    superseded_by: None,
};

static CTG_ID: OnceLock<RulesetId> = OnceLock::new();
static CTG_VERSION: OnceLock<RulesetVersion> = OnceLock::new();
static CTG_DATES: OnceLock<EffectiveDateBound> = OnceLock::new();
static CTG_STAGES: [LifecycleStage; 2] = [LifecycleStage::RawMaterials, LifecycleStage::Production];

/// Generic cradle-to-gate CO₂e ruleset (raw materials + production stages).
///
/// Use this when the delegated act does not mandate a specific lifecycle model.
/// Battery CFB and other full-lifecycle calculations use their own sector ruleset.
pub struct CradleToGateRuleset;

impl Ruleset for CradleToGateRuleset {
    fn id(&self) -> &RulesetId {
        CTG_ID.get_or_init(|| RulesetId("co2e-cradle-to-gate".into()))
    }

    fn version(&self) -> &RulesetVersion {
        CTG_VERSION.get_or_init(|| RulesetVersion("1.0.0".into()))
    }

    fn effective_dates(&self) -> &EffectiveDateBound {
        CTG_DATES.get_or_init(|| {
            EffectiveDateBound::open(NaiveDate::from_ymd_opt(2021, 1, 1).expect("valid date"))
        })
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &CTG_BASIS
    }
}

impl Co2eRuleset for CradleToGateRuleset {
    fn declared_stages(&self) -> &[LifecycleStage] {
        &CTG_STAGES
    }
}
