//! The index calculation itself.

use serde::{Deserialize, Serialize};

use super::parameters::{MAX_SCORE, MIN_SCORE, PriorityPartScores, RepairabilityIndexInputs};
use super::thresholds::RepairabilityIndexRuleset;
use crate::error::CalcError;

/// Repairability class shown on the energy label. Annex II, Table 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairabilityClass {
    A,
    B,
    C,
    D,
    E,
}

/// Result of an index calculation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairabilityIndexResult {
    /// The index R, on the regulation's 1,00–5,00 scale.
    pub index: f64,
    /// The class R falls into.
    pub class: RepairabilityClass,
    /// Aggregated SDD.
    pub disassembly_depth: f64,
    /// Aggregated SF.
    pub fasteners: f64,
    /// Aggregated ST.
    pub tools: f64,
    /// Whether the foldable weight set was applied.
    pub foldable: bool,
}

fn check_range(label: &str, score: u8) -> Result<(), CalcError> {
    if !(MIN_SCORE..=MAX_SCORE).contains(&score) {
        return Err(CalcError::InvalidInput(format!(
            "{label} must be {MIN_SCORE}–{MAX_SCORE} per Annex IV point 5, got {score}"
        )));
    }
    Ok(())
}

fn check_parts(label: &str, parts: &PriorityPartScores) -> Result<(), CalcError> {
    for (i, s) in parts.each().iter().enumerate() {
        check_range(&format!("{label} priority part {i}"), *s)?;
    }
    if let Some(fm) = parts.folding_mechanism {
        check_range(&format!("{label} folding mechanism"), fm)?;
    }
    Ok(())
}

/// Weighted aggregation of one part-level parameter.
fn aggregate(parts: &PriorityPartScores, ruleset: &dyn RepairabilityIndexRuleset) -> f64 {
    let w = ruleset.part_weights(parts.is_foldable());
    let mut total = f64::from(parts.battery) * w.battery
        + f64::from(parts.display_assembly) * w.display_assembly
        + f64::from(parts.back_cover) * w.back_cover;

    // The six minor parts share one weight.
    for s in [
        parts.front_camera,
        parts.rear_camera,
        parts.charging_port,
        parts.mechanical_button,
        parts.microphone,
        parts.speaker,
    ] {
        total += f64::from(s) * w.minor_part;
    }

    if let (Some(score), Some(weight)) = (parts.folding_mechanism, w.folding_mechanism) {
        total += f64::from(score) * weight;
    }
    total
}

/// Calculate the repairability index and class for a smartphone or slate tablet.
///
/// # Errors
/// [`CalcError::InvalidInput`] if any score is outside 1–5.
/// [`CalcError::CrossFieldViolation`] if the three part-level parameters
/// disagree about whether the product is foldable — foldability is a property of
/// the product and selects the weight set, so it cannot differ between SDD, SF
/// and ST.
pub fn calculate(
    inputs: &RepairabilityIndexInputs,
    ruleset: &dyn RepairabilityIndexRuleset,
) -> Result<RepairabilityIndexResult, CalcError> {
    check_parts("disassembly depth", &inputs.disassembly_depth)?;
    check_parts("fasteners", &inputs.fasteners)?;
    check_parts("tools", &inputs.tools)?;
    check_range("spare parts", inputs.spare_parts)?;
    check_range("software updates", inputs.software_updates)?;
    check_range("repair information", inputs.repair_information)?;

    let foldable = inputs.disassembly_depth.is_foldable();
    if inputs.fasteners.is_foldable() != foldable || inputs.tools.is_foldable() != foldable {
        return Err(CalcError::CrossFieldViolation(
            "foldability must agree across disassembly depth, fasteners and tools — \
             it is a product property that selects the Annex IV weight set"
                .into(),
        ));
    }

    let sdd = aggregate(&inputs.disassembly_depth, ruleset);
    let sf = aggregate(&inputs.fasteners, ruleset);
    let st = aggregate(&inputs.tools, ruleset);

    let w = ruleset.weights();
    let index = sdd * w.disassembly_depth
        + sf * w.fasteners
        + st * w.tools
        + f64::from(inputs.spare_parts) * w.spare_parts
        + f64::from(inputs.software_updates) * w.software_updates
        + f64::from(inputs.repair_information) * w.repair_information;

    let b = ruleset.class_boundaries();
    let class = if index >= b.a {
        RepairabilityClass::A
    } else if index >= b.b {
        RepairabilityClass::B
    } else if index >= b.c {
        RepairabilityClass::C
    } else if index >= b.d {
        RepairabilityClass::D
    } else {
        RepairabilityClass::E
    };

    Ok(RepairabilityIndexResult {
        index,
        class,
        disassembly_depth: sdd,
        fasteners: sf,
        tools: st,
        foldable,
    })
}
