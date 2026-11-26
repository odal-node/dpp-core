//! Tests of the `calculator` module.

use super::super::*;
use crate::repairability::thresholds::SimplifiedRepairabilityHeuristic;
use chrono::NaiveDate;

/// A clock inside SimplifiedRepairabilityHeuristic's effective period
/// (open from 2025-06-20).
fn in_force() -> AssessmentClock {
    AssessmentClock::placed_on(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
}

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
        calculate(&inputs, &SimplifiedRepairabilityHeuristic, in_force()),
        Err(CalcError::InvalidInput(_))
    ));
}
