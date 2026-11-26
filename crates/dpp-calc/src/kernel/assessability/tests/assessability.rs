//! Tests of the `assessability` module.

use super::super::*;
use chrono::NaiveDate;

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

#[test]
fn the_four_non_assessed_outcomes_are_distinguishable() {
    let outcomes: [Assessability<&str>; 4] = [
        Assessability::NotYetInForce {
            ruleset_id: "a",
            applies_from: day(2031, 8, 18),
        },
        Assessability::Undetermined {
            ruleset_id: "b",
            empowerment: "Art. 10(5)",
        },
        Assessability::Expired {
            ruleset_id: "c",
            until: day(2026, 12, 31),
            superseded_by: Some("c-v2"),
        },
        Assessability::OutOfScope,
    ];
    for (i, a) in outcomes.iter().enumerate() {
        assert!(!a.is_assessed());
        for (j, b) in outcomes.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "outcomes {i} and {j} must not compare equal");
            }
        }
    }
}

#[test]
fn assessed_unwraps_only_the_assessed_variant() {
    assert_eq!(Assessability::Assessed("x").assessed(), Some("x"));
    assert_eq!(Assessability::<&str>::OutOfScope.assessed(), None);
    assert!(Assessability::Assessed("x").is_assessed());
}
