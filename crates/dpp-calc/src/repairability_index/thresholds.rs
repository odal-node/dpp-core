//! The EU 2023/1669 ruleset: weights, class boundaries and regulatory basis.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ruleset::{EffectiveDateBound, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

/// Parameter weights for the index. Annex IV point 5:
/// `R = SDD*0,25 + SF*0,15 + ST*0,15 + SSP*0,15 + SSU*0,15 + SRI*0,15`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexWeights {
    pub disassembly_depth: f64,
    pub fasteners: f64,
    pub tools: f64,
    pub spare_parts: f64,
    pub software_updates: f64,
    pub repair_information: f64,
}

/// Per-priority-part weights used to aggregate SDD, SF and ST.
///
/// Annex IV point 5 gives two sets: one for products without a hinge or
/// mechanical display folding mechanism, one for products with. Each sums to 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartWeights {
    pub battery: f64,
    pub display_assembly: f64,
    pub back_cover: f64,
    /// Applied to each of the six minor parts (cameras, port, button, mic, speaker).
    pub minor_part: f64,
    /// Applied to the hinge / folding mechanism; `None` when not applicable.
    pub folding_mechanism: Option<f64>,
}

/// Minimum index value for each repairability class. Annex II, Table 4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexClassBoundaries {
    /// A: R ≥ 4,00.
    pub a: f64,
    /// B: 4,00 > R ≥ 3,35.
    pub b: f64,
    /// C: 3,35 > R ≥ 2,55.
    pub c: f64,
    /// D: 2,55 > R ≥ 1,75. Below this, class E.
    pub d: f64,
}

/// Ruleset for the enacted EU repairability index.
///
/// Extends [`Ruleset`]; the index-specific data is in the four methods below.
pub trait RepairabilityIndexRuleset: Ruleset {
    fn weights(&self) -> &IndexWeights;
    fn part_weights(&self, foldable: bool) -> &PartWeights;
    fn class_boundaries(&self) -> &IndexClassBoundaries;
}

static WEIGHTS: IndexWeights = IndexWeights {
    disassembly_depth: 0.25,
    fasteners: 0.15,
    tools: 0.15,
    spare_parts: 0.15,
    software_updates: 0.15,
    repair_information: 0.15,
};

static PART_WEIGHTS_RIGID: PartWeights = PartWeights {
    battery: 0.30,
    display_assembly: 0.30,
    back_cover: 0.10,
    minor_part: 0.05,
    folding_mechanism: None,
};

static PART_WEIGHTS_FOLDABLE: PartWeights = PartWeights {
    battery: 0.25,
    display_assembly: 0.25,
    back_cover: 0.09,
    minor_part: 0.04,
    folding_mechanism: Some(0.17),
};

static CLASS_BOUNDARIES: IndexClassBoundaries = IndexClassBoundaries {
    a: 4.00,
    b: 3.35,
    c: 2.55,
    d: 1.75,
};

static BASIS: RegulatoryBasis = RegulatoryBasis {
    regulation: "EU 2023/1669",
    article: "Annex IV point 5 (calculation method); Annex II Table 4 (class boundaries)",
    standard: Some("EN 45554:2020"),
    technical_study: Some("JRC128672"),
    source_url: Some("https://eur-lex.europa.eu/eli/reg_del/2023/1669/oj/eng"),
    superseded_by: None,
};

static ID: RulesetId = RulesetId("eu-2023-1669-repairability-index");
static VERSION: RulesetVersion = RulesetVersion("1.0.0");
static EFFECTIVE: std::sync::OnceLock<EffectiveDateBound> = std::sync::OnceLock::new();

/// The enacted EU repairability index for smartphones and slate tablets,
/// applicable from 2025-06-20.
///
/// Unlike [`crate::repairability::SimplifiedRepairabilityHeuristic`], the output
/// of this ruleset **is** the regulatory repairability class shown on the energy
/// label. The two are not comparable: this index runs 1,00–5,00, the heuristic
/// 0–10.
pub struct Eu2023_1669Ruleset;

impl Ruleset for Eu2023_1669Ruleset {
    fn id(&self) -> &RulesetId {
        &ID
    }

    fn version(&self) -> &RulesetVersion {
        &VERSION
    }

    fn effective_dates(&self) -> &EffectiveDateBound {
        EFFECTIVE.get_or_init(|| {
            EffectiveDateBound::open(NaiveDate::from_ymd_opt(2025, 6, 20).expect("valid date"))
        })
    }

    fn regulatory_basis(&self) -> &RegulatoryBasis {
        &BASIS
    }
}

impl RepairabilityIndexRuleset for Eu2023_1669Ruleset {
    fn weights(&self) -> &IndexWeights {
        &WEIGHTS
    }

    fn part_weights(&self, foldable: bool) -> &PartWeights {
        if foldable {
            &PART_WEIGHTS_FOLDABLE
        } else {
            &PART_WEIGHTS_RIGID
        }
    }

    fn class_boundaries(&self) -> &IndexClassBoundaries {
        &CLASS_BOUNDARIES
    }
}
