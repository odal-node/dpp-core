//! Plausibility checks on a textile's declared facts.

use alloc::{format, vec::Vec};

use super::super::{LintFinding, LintSeverity};

/// Borrowing view over the textile fields these lints inspect.
#[derive(Debug, Clone, Copy)]
pub struct TextileLintInput<'a> {
    pub durability_score: Option<f64>,
    pub expected_wash_cycles: Option<u32>,
    pub repair_count: Option<u32>,
    pub repair_history_url: Option<&'a str>,
    pub prior_use_cycles: Option<u32>,
    pub reuse_condition: Option<&'a str>,
    pub repair_score: Option<f64>,
    pub disassembly_instructions: Option<&'a str>,
    pub spare_parts_available: Option<bool>,
    pub microplastic_shedding_mg_per_wash: Option<f64>,
    pub fibres: &'a [&'a str],
}

const HIGH_DURABILITY_THRESHOLD: f64 = 8.0;
const LOW_WASH_CYCLES_THRESHOLD: u32 = 10;

/// Cross-field plausibility: a garment declared highly durable (≥8/10) that
/// is also expected to degrade within under 10 wash cycles is self-contradictory.
#[must_use]
pub fn durability_wash_cycles_mismatch(input: &TextileLintInput<'_>) -> Option<LintFinding> {
    let durability = input.durability_score?;
    let cycles = input.expected_wash_cycles?;
    if durability < HIGH_DURABILITY_THRESHOLD || cycles >= LOW_WASH_CYCLES_THRESHOLD {
        return None;
    }
    Some(LintFinding {
        code: "textile.durability_wash_cycles_mismatch",
        field: "expectedWashCycles",
        severity: LintSeverity::Notice,
        message: format!(
            "durabilityScore ({durability:.1}/10) is high but expectedWashCycles ({cycles}) is low — intended?"
        ),
    })
}

/// Claim-without-evidence check: a positive `repairCount` with no
/// `repairHistoryUrl` asserts repairs happened with nothing to point to.
#[must_use]
pub fn repair_count_without_history(input: &TextileLintInput<'_>) -> Option<LintFinding> {
    let count = input.repair_count?;
    if count == 0 || input.repair_history_url.is_some() {
        return None;
    }
    Some(LintFinding {
        code: "textile.repair_count_without_history",
        field: "repairCount",
        severity: LintSeverity::Notice,
        message: format!("repairCount is {count} but repairHistoryUrl is absent — intended?"),
    })
}

/// Claim-without-evidence check: a positive `priorUseCycles` (this is not a
/// new item) with no `reuseCondition` grade omits the condition a buyer of a
/// used item would expect.
#[must_use]
pub fn prior_use_without_reuse_condition(input: &TextileLintInput<'_>) -> Option<LintFinding> {
    let cycles = input.prior_use_cycles?;
    if cycles == 0 || input.reuse_condition.is_some() {
        return None;
    }
    Some(LintFinding {
        code: "textile.prior_use_without_reuse_condition",
        field: "priorUseCycles",
        severity: LintSeverity::Notice,
        message: format!("priorUseCycles is {cycles} but reuseCondition is absent — intended?"),
    })
}

const HIGH_REPAIR_SCORE_THRESHOLD: f64 = 8.0;

/// Claim-without-evidence check: a high `repairScore` (≥8/10) with neither
/// disassembly instructions nor confirmed spare-parts availability asserts
/// repairability with nothing backing it.
#[must_use]
pub fn repair_score_high_without_support(input: &TextileLintInput<'_>) -> Option<LintFinding> {
    let score = input.repair_score?;
    if score < HIGH_REPAIR_SCORE_THRESHOLD
        || input.disassembly_instructions.is_some()
        || input.spare_parts_available == Some(true)
    {
        return None;
    }
    Some(LintFinding {
        code: "textile.repair_score_high_without_support",
        field: "repairScore",
        severity: LintSeverity::Notice,
        message: format!(
            "repairScore ({score:.1}/10) is high but neither disassemblyInstructions nor \
             sparePartsAvailable is declared — intended?"
        ),
    })
}

const KNOWN_NATURAL_FIBRES: &[&str] = &[
    "cotton", "wool", "silk", "linen", "hemp", "jute", "cashmere", "alpaca", "mohair", "flax",
    "ramie", "angora",
];

fn is_known_natural(fibre: &str) -> bool {
    let normalized = fibre.trim();
    KNOWN_NATURAL_FIBRES
        .iter()
        .any(|f| normalized.eq_ignore_ascii_case(f))
}

/// Physics check: microplastic fibre shedding (ISO/DIS 4484) is a
/// synthetic-polymer phenomenon — natural fibres like cotton or wool do not
/// shed microplastics on washing. Fires only when *every* declared fibre is
/// recognised as natural, to avoid false positives on unrecognised or blended
/// synthetic names.
#[must_use]
pub fn microplastic_shedding_without_synthetic_fibre(
    input: &TextileLintInput<'_>,
) -> Option<LintFinding> {
    let shedding = input.microplastic_shedding_mg_per_wash?;
    if !shedding.is_finite() || shedding <= 0.0 {
        return None;
    }
    if input.fibres.is_empty() || !input.fibres.iter().all(|f| is_known_natural(f)) {
        return None;
    }
    Some(LintFinding {
        code: "textile.microplastic_shedding_without_synthetic_fibre",
        field: "microplasticSheddingMgPerWash",
        severity: LintSeverity::Notice,
        message: format!(
            "microplasticSheddingMgPerWash ({shedding:.2} mg) is declared but every fibre in \
             fibreComposition is a natural fibre — intended?"
        ),
    })
}

/// Run every textile plausibility lint and collect the findings.
#[must_use]
pub fn lint_textile(input: &TextileLintInput<'_>) -> Vec<LintFinding> {
    let mut out = Vec::new();
    out.extend(durability_wash_cycles_mismatch(input));
    out.extend(repair_count_without_history(input));
    out.extend(prior_use_without_reuse_condition(input));
    out.extend(repair_score_high_without_support(input));
    out.extend(microplastic_shedding_without_synthetic_fibre(input));
    out
}
