//! Threshold tables and product-category rulesets for the **simplified
//! repairability heuristic**.
//!
//! ⚠️ This is **not** the enacted EU 2023/1669 (smartphones/tablets) Annex IV
//! repairability index. That methodology uses a different parameter set
//! (incl. Fasteners & Tools), a 1–5 per-class scale, a priority-part dimension,
//! and its own class boundaries. The model here is a transparent six-factor
//! 0–2 heuristic; its output is a heuristic band, not a regulatory class.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::CalcError;
use crate::repairability::parameters::RepairabilityInputs;
use crate::ruleset::{EffectiveDateBound, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

/// Weight coefficient for each heuristic parameter.
///
/// Weights must sum to 1.0. Each parameter score (0–2) is multiplied by its
/// weight and by 5.0 to produce a contribution to the 0–10 numeric score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairabilityWeights {
    pub disassembly: f64,
    pub spare_parts: f64,
    pub repair_info: f64,
    pub diagnostic_tools: f64,
    pub software_updatability: f64,
    pub customer_support: f64,
}

/// Minimum numeric score (out of 10) required for each letter grade.
///
/// Grade E is assigned when the score is below `d`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairabilityThresholds {
    /// Minimum score for grade A (highest).
    pub a: f64,
    pub b: f64,
    pub c: f64,
    /// Minimum score for grade D. Below this value → grade E.
    pub d: f64,
}

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

// ---------------------------------------------------------------------------
// Simplified repairability heuristic (applied to smartphones/tablets today)
// ---------------------------------------------------------------------------
//
// NON-REGULATORY. This is a transparent six-factor 0–2 indicator, NOT the
// enacted EU 2023/1669 Annex IV repairability index (which uses a different
// parameter set incl. Fasteners & Tools, a 1–5 per-class scale, a priority-part
// dimension, and its own class boundaries — see docs/audit H-1). The weights and
// band thresholds below are heuristic design choices; they are deliberately
// NOT pinned to any OJ annex, and the output must not be presented as a
// regulatory repairability class.
// ---------------------------------------------------------------------------
static SMARTPHONE_WEIGHTS: RepairabilityWeights = RepairabilityWeights {
    disassembly: 0.25,
    spare_parts: 0.15,
    repair_info: 0.15,
    diagnostic_tools: 0.15,
    software_updatability: 0.15,
    customer_support: 0.15,
};

// Heuristic band boundaries (a=8.5, b=7.0, c=5.5, d=4.0) — design choices, not a
// regulatory grade table. Below `d` ⇒ band E.
static SMARTPHONE_THRESHOLDS: RepairabilityThresholds = RepairabilityThresholds {
    a: 8.5,
    b: 7.0,
    c: 5.5,
    d: 4.0,
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
        &SMARTPHONE_THRESHOLDS
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

// ---------------------------------------------------------------------------
// Laptops — reserved; product-specific delegated act expected ~2027
// ---------------------------------------------------------------------------

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

static LAPTOP_THRESHOLDS: RepairabilityThresholds = RepairabilityThresholds {
    a: 8.5,
    b: 7.0,
    c: 5.5,
    d: 4.0,
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
        &LAPTOP_THRESHOLDS
    }
}

// ---------------------------------------------------------------------------
// Electronic displays (TVs, monitors) — stub; ESPR delegated act expected
// ---------------------------------------------------------------------------
//
// EU 2019/2021 covers ecodesign for electronic displays. An ESPR-era repairability
// delegated act is expected. Weights below are placeholder (uniform) pending the
// official annex. Effective-date sentinel: 2100-01-01 blocks runtime use.

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

// ---------------------------------------------------------------------------
// Washing machines / washer-dryers — stub; ESPR delegated act expected
// ---------------------------------------------------------------------------
//
// EU 2021/341 includes repairability requirements for washing machines. An
// ESPR-era repairability index delegated act is expected ~2026. Weights and
// thresholds below are placeholder pending the official annex.

pub struct WashingMachineRuleset;

static WASHING_WEIGHTS: RepairabilityWeights = RepairabilityWeights {
    disassembly: 0.25,
    spare_parts: 0.20,
    repair_info: 0.20,
    diagnostic_tools: 0.15,
    software_updatability: 0.10,
    customer_support: 0.10,
};

static WASHING_THRESHOLDS: RepairabilityThresholds = RepairabilityThresholds {
    a: 8.5,
    b: 7.0,
    c: 5.5,
    d: 4.0,
};

static WASHING_BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "pending — ESPR washing machine repairability delegated act (expected ~2026)",
    article: "TBD",
    standard: Some("EN 45554:2021"),
    technical_study: None,
    source_url: None,
    superseded_by: None,
};

static WASHING_RULESET_ID: std::sync::OnceLock<RulesetId> = std::sync::OnceLock::new();
static WASHING_RULESET_VERSION: std::sync::OnceLock<RulesetVersion> = std::sync::OnceLock::new();
static WASHING_EFFECTIVE_DATES: std::sync::OnceLock<EffectiveDateBound> =
    std::sync::OnceLock::new();

impl Ruleset for WashingMachineRuleset {
    fn id(&self) -> &RulesetId {
        WASHING_RULESET_ID.get_or_init(|| RulesetId("washing-machine-repairability".into()))
    }

    fn version(&self) -> &RulesetVersion {
        WASHING_RULESET_VERSION.get_or_init(|| RulesetVersion("0.0.0-stub".into()))
    }

    fn effective_dates(&self) -> &EffectiveDateBound {
        WASHING_EFFECTIVE_DATES.get_or_init(|| {
            EffectiveDateBound::open(NaiveDate::from_ymd_opt(2100, 1, 1).expect("valid date"))
        })
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &WASHING_BASIS
    }
}

impl RepairabilityRuleset for WashingMachineRuleset {
    fn weights(&self) -> &RepairabilityWeights {
        &WASHING_WEIGHTS
    }

    fn thresholds(&self) -> &RepairabilityThresholds {
        &WASHING_THRESHOLDS
    }
}

#[cfg(test)]
mod stub_ruleset_tests {
    use super::*;
    use crate::error::CalcError;
    use crate::repairability::calculate;

    fn valid_inputs() -> RepairabilityInputs {
        RepairabilityInputs {
            disassembly: 2,
            spare_parts: 2,
            repair_info: 2,
            diagnostic_tools: 2,
            software_updatability: 2,
            customer_support: 2,
        }
    }

    #[test]
    fn stub_rulesets_expose_consistent_metadata() {
        let rulesets: [&dyn RepairabilityRuleset; 3] =
            [&LaptopRuleset, &DisplaysRuleset, &WashingMachineRuleset];
        for rs in rulesets {
            let w = rs.weights();
            let sum = w.disassembly
                + w.spare_parts
                + w.repair_info
                + w.diagnostic_tools
                + w.software_updatability
                + w.customer_support;
            assert!((sum - 1.0).abs() < 1e-9, "weights must sum to 1.0");
            assert_eq!(rs.thresholds().a, 8.5);
            assert!(!rs.id().0.is_empty());
            assert!(!rs.version().0.is_empty());
            assert!(!rs.regulatory_basis().regulation.is_empty());
            // 2100 sentinel: these acts are not yet in force.
            assert!(
                !rs.effective_dates()
                    .is_active_on(chrono::Utc::now().date_naive())
            );
        }
    }

    #[test]
    fn calculating_with_a_not_yet_in_force_ruleset_is_rejected() {
        // Laptop/Displays/Washing all carry the 2100 effective-date sentinel,
        // so calculate() must refuse them via the RulesetExpired guard.
        for result in [
            calculate(&valid_inputs(), &LaptopRuleset),
            calculate(&valid_inputs(), &DisplaysRuleset),
            calculate(&valid_inputs(), &WashingMachineRuleset),
        ] {
            assert!(matches!(result, Err(CalcError::RulesetExpired { .. })));
        }
    }
}
