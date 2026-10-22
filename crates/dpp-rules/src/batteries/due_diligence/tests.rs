//! Art. 47 and Art. 48(1): the date, the two exemptions, and the missing facts.

use super::*;
use crate::common::date::CalendarDate;

const DAY_BEFORE: CalendarDate = CalendarDate::new(2027, 8, 17);

#[test]
fn the_date_is_inclusive() {
    let original = DueDiligenceSecondLife::None;
    let subject = Some(OperatorScope::Subject);
    assert_eq!(
        due_diligence_scope(Some(DAY_BEFORE), original, subject),
        DueDiligenceScope::NotYetBinding
    );
    assert_eq!(
        due_diligence_scope(Some(DUE_DILIGENCE_REQUIRED_FROM), original, subject),
        DueDiligenceScope::Required
    );
}

#[test]
fn a_missing_date_is_not_an_exemption() {
    let scope = due_diligence_scope(
        None,
        DueDiligenceSecondLife::None,
        Some(OperatorScope::BelowTurnoverThreshold),
    );
    assert_eq!(scope, DueDiligenceScope::PlacingDateUnknown);
    assert!(scope.is_undetermined());
}

#[test]
fn a_missing_declaration_is_not_an_exemption() {
    let scope = due_diligence_scope(
        Some(DUE_DILIGENCE_REQUIRED_FROM),
        DueDiligenceSecondLife::None,
        None,
    );
    assert_eq!(scope, DueDiligenceScope::OperatorScopeUndeclared);
    assert!(scope.is_undetermined());
}

#[test]
fn the_declaration_is_not_asked_for_before_the_date() {
    // A battery placed in March 2027 owes no report, so whether its operator
    // would have been subject is not a question with any consequence.
    assert_eq!(
        due_diligence_scope(Some(DAY_BEFORE), DueDiligenceSecondLife::None, None),
        DueDiligenceScope::NotYetBinding
    );
}

#[test]
fn the_turnover_limb_exempts_on_the_declaration() {
    assert_eq!(
        due_diligence_scope(
            Some(DUE_DILIGENCE_REQUIRED_FROM),
            DueDiligenceSecondLife::None,
            Some(OperatorScope::BelowTurnoverThreshold),
        ),
        DueDiligenceScope::ExemptBelowTurnoverThreshold
    );
}

#[test]
fn the_second_life_limb_needs_no_date_and_no_declaration() {
    // Art. 77(7) gives such a battery a new passport, whose placing date falls
    // after its first one. Without this limb every second-life passport placed
    // from August 2027 would owe a report its operator is not required to write.
    for placed in [None, Some(DAY_BEFORE), Some(DUE_DILIGENCE_REQUIRED_FROM)] {
        for operator in [None, Some(OperatorScope::Subject)] {
            assert_eq!(
                due_diligence_scope(
                    placed,
                    DueDiligenceSecondLife::PlacedBeforeOperations,
                    operator
                ),
                DueDiligenceScope::ExemptSecondLife,
                "{placed:?} {operator:?}"
            );
        }
    }
}

#[test]
fn the_wire_spelling_round_trips_and_nothing_else_parses() {
    for scope in [
        OperatorScope::Subject,
        OperatorScope::BelowTurnoverThreshold,
    ] {
        assert_eq!(OperatorScope::from_wire(scope.wire_str()), Some(scope));
    }
    for bad in ["", "Subject", "below", "exempt", "belowturnoverthreshold"] {
        assert_eq!(OperatorScope::from_wire(bad), None, "{bad:?}");
    }
}
