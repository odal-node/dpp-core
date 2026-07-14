//! The simplified repairability heuristic calculation: inputs → A–E band.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::parameters::RepairabilityInputs;
use super::thresholds::RepairabilityRuleset;
use crate::error::CalcError;
use crate::receipt::{CalculationReceipt, input_hash, jcs_hash};

/// A–E heuristic band from the simplified repairability indicator.
///
/// Not a regulatory repairability class — see the module-level note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairabilityClass {
    A,
    B,
    C,
    D,
    E,
}

impl RepairabilityClass {
    /// Numeric ordinal for interop (A=5, B=4, C=3, D=2, E=1).
    pub fn as_ordinal(self) -> u8 {
        match self {
            Self::A => 5,
            Self::B => 4,
            Self::C => 3,
            Self::D => 2,
            Self::E => 1,
        }
    }
}

impl std::fmt::Display for RepairabilityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::A => "A",
                Self::B => "B",
                Self::C => "C",
                Self::D => "D",
                Self::E => "E",
            }
        )
    }
}

/// Weighted contribution of each heuristic parameter to the final 0–10 score.
///
/// Each field equals `parameter_value × weight × 5.0`. The sum of all six fields
/// equals [`RepairabilityResult::numeric_score`], allowing an auditor to trace
/// any individual parameter's share of the band without re-running the calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterContributions {
    pub disassembly: f64,
    pub spare_parts: f64,
    pub repair_info: f64,
    pub diagnostic_tools: f64,
    pub software_updatability: f64,
    pub customer_support: f64,
}

/// Output of the simplified repairability heuristic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairabilityResult {
    /// A–E heuristic band (not a regulatory class).
    pub class: RepairabilityClass,
    /// Continuous numeric score in `[0.0, 10.0]`.
    pub numeric_score: f64,
    /// Per-parameter weighted contributions — sum equals `numeric_score`.
    pub contributions: ParameterContributions,
    /// Proof-of-calculation receipt.
    pub receipt: CalculationReceipt,
}

/// Calculate the simplified repairability heuristic band for one product.
///
/// The result is a non-regulatory heuristic, not the EU 2023/1669 class.
/// Returns `Err` if any parameter value is outside `[0, 2]` or if the
/// ruleset's effective period does not cover today's date.
pub fn calculate(
    inputs: &RepairabilityInputs,
    ruleset: &dyn RepairabilityRuleset,
) -> Result<RepairabilityResult, CalcError> {
    validate_inputs(inputs)?;
    ruleset.validate_cross_fields(inputs)?;

    ruleset
        .effective_dates()
        .ensure_active_on(ruleset.id(), Utc::now().date_naive())?;

    let w = ruleset.weights();
    let scale = 5.0; // scale 0–2 ordinals to 0–10

    let contributions = ParameterContributions {
        disassembly: f64::from(inputs.disassembly) * w.disassembly * scale,
        spare_parts: f64::from(inputs.spare_parts) * w.spare_parts * scale,
        repair_info: f64::from(inputs.repair_info) * w.repair_info * scale,
        diagnostic_tools: f64::from(inputs.diagnostic_tools) * w.diagnostic_tools * scale,
        software_updatability: f64::from(inputs.software_updatability)
            * w.software_updatability
            * scale,
        customer_support: f64::from(inputs.customer_support) * w.customer_support * scale,
    };

    let numeric_score = contributions.disassembly
        + contributions.spare_parts
        + contributions.repair_info
        + contributions.diagnostic_tools
        + contributions.software_updatability
        + contributions.customer_support;

    let t = ruleset.thresholds();
    let class = if numeric_score >= t.a {
        RepairabilityClass::A
    } else if numeric_score >= t.b {
        RepairabilityClass::B
    } else if numeric_score >= t.c {
        RepairabilityClass::C
    } else if numeric_score >= t.d {
        RepairabilityClass::D
    } else {
        RepairabilityClass::E
    };

    let output_hash = jcs_hash(&(numeric_score, class.as_ordinal()))?;

    let receipt = CalculationReceipt::new(
        input_hash(inputs)?,
        ruleset.id().0.as_str(),
        ruleset.version().0.as_str(),
    )
    .with_output_hash(output_hash);

    Ok(RepairabilityResult {
        class,
        numeric_score,
        contributions,
        receipt,
    })
}

fn validate_inputs(inputs: &RepairabilityInputs) -> Result<(), CalcError> {
    for (name, val) in [
        ("disassembly", inputs.disassembly),
        ("spare_parts", inputs.spare_parts),
        ("repair_info", inputs.repair_info),
        ("diagnostic_tools", inputs.diagnostic_tools),
        ("software_updatability", inputs.software_updatability),
        ("customer_support", inputs.customer_support),
    ] {
        if val > 2 {
            return Err(CalcError::InvalidInput(format!(
                "{name} must be 0, 1, or 2; got {val}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repairability::thresholds::SimplifiedRepairabilityHeuristic;

    #[test]
    fn class_display_and_ordinal_for_all_grades() {
        for (class, letter, ordinal) in [
            (RepairabilityClass::A, "A", 5),
            (RepairabilityClass::B, "B", 4),
            (RepairabilityClass::C, "C", 3),
            (RepairabilityClass::D, "D", 2),
            (RepairabilityClass::E, "E", 1),
        ] {
            assert_eq!(class.to_string(), letter);
            assert_eq!(class.as_ordinal(), ordinal);
        }
    }

    #[test]
    fn out_of_range_parameter_is_rejected() {
        let inputs = RepairabilityInputs {
            disassembly: 3, // out of [0, 2]
            spare_parts: 1,
            repair_info: 1,
            diagnostic_tools: 1,
            software_updatability: 1,
            customer_support: 1,
        };
        assert!(matches!(
            calculate(&inputs, &SimplifiedRepairabilityHeuristic),
            Err(CalcError::InvalidInput(_))
        ));
    }
}
