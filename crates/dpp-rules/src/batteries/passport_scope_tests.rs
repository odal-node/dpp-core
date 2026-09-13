//! Art. 77(1) passport scope — the three categories it names, the threshold it
//! attaches to one of them, and the date it all starts from.

use super::passport_scope::{
    INDUSTRIAL_PASSPORT_THRESHOLD_KWH, PASSPORT_REQUIRED_FROM, PassportScope, passport_scope,
};
use crate::common::date::CalendarDate;

/// After the obligation begins, so category and threshold are what decide.
const AFTER: CalendarDate = CalendarDate::new(2027, 6, 1);

#[test]
fn the_three_categories_art_77_1_names_owe_a_passport() {
    assert_eq!(passport_scope("ev", None, AFTER), PassportScope::Required);
    assert_eq!(passport_scope("lmt", None, AFTER), PassportScope::Required);
    assert_eq!(
        passport_scope("industrial", Some(50.0), AFTER),
        PassportScope::Required
    );
}

/// The half of the defect that let a passport claim an obligation that does not
/// exist. Portable and SLI are named nowhere in Art. 77(1).
#[test]
fn portable_and_sli_owe_no_passport_at_all() {
    for t in ["portable", "sli", "starting-lighting-ignition"] {
        assert_eq!(
            passport_scope(t, Some(500.0), AFTER),
            PassportScope::NotCovered,
            "{t} is outside Art. 77(1) whatever its capacity"
        );
    }
}

/// The other half: an industrial battery at or below 2 kWh is exempt, and was
/// being held to all 38 industrial data points.
///
/// "greater than 2 kWh" is strict, so exactly 2.0 owes nothing.
#[test]
fn an_industrial_battery_at_or_below_the_threshold_is_exempt() {
    assert_eq!(
        passport_scope("industrial", Some(1.0), AFTER),
        PassportScope::BelowThreshold
    );
    assert_eq!(
        passport_scope("industrial", Some(INDUSTRIAL_PASSPORT_THRESHOLD_KWH), AFTER),
        PassportScope::BelowThreshold,
        "'greater than 2 kWh' excludes exactly 2 kWh"
    );
    assert_eq!(
        passport_scope("industrial", Some(2.000_001), AFTER),
        PassportScope::Required,
        "and the first value above it is in scope"
    );
}

/// 🚨 A missing capacity must never read as an exemption.
///
/// The obligation turns on a number the record does not carry. Answering
/// `NotCovered` would exempt a battery on the strength of an absent field, which
/// is the failure direction this whole module exists to prevent.
#[test]
fn an_industrial_battery_with_no_capacity_is_undetermined_not_exempt() {
    let scope = passport_scope("industrial", None, AFTER);
    assert_eq!(scope, PassportScope::CapacityUnknown);
    assert!(!scope.is_required());
    assert!(
        scope.is_undetermined(),
        "the caller must be able to route this to 'ask for the capacity'"
    );
    assert_ne!(
        scope,
        PassportScope::NotCovered,
        "an unstated capacity is not an exemption"
    );
}

/// NaN compares false against every bound. Without the explicit fall-through it
/// would slip past `> threshold` and land in the `<=` arm, reporting an
/// unusable number as a battery below the threshold.
#[test]
fn a_nan_capacity_is_undetermined_rather_than_below_the_threshold() {
    assert_eq!(
        passport_scope("industrial", Some(f64::NAN), AFTER),
        PassportScope::CapacityUnknown
    );
}

/// The date gate. A battery placed before 18 Feb 2027 owes no passport, and
/// reporting one as non-compliant would be a retroactive finding — the same
/// error `art8_phase_for` refuses for Art. 8.
#[test]
fn nothing_is_binding_before_18_february_2027() {
    let day_before = CalendarDate::new(2027, 2, 17);
    for (t, cap) in [("ev", None), ("lmt", None), ("industrial", Some(50.0))] {
        assert_eq!(
            passport_scope(t, cap, day_before),
            PassportScope::NotYetBinding,
            "{t} placed the day before the obligation begins"
        );
        assert_eq!(
            passport_scope(t, cap, PASSPORT_REQUIRED_FROM),
            PassportScope::Required,
            "{t} on the first day it binds"
        );
    }
}

/// A category outside Art. 77(1) is outside it before the date too — "never"
/// and "not yet" must not be swapped, which is why the category test runs
/// before the date test.
#[test]
fn out_of_scope_beats_not_yet_binding() {
    assert_eq!(
        passport_scope("portable", None, CalendarDate::new(2020, 1, 1)),
        PassportScope::NotCovered,
        "portable is NotCovered, never NotYetBinding"
    );
    assert_eq!(
        passport_scope("industrial", Some(0.5), CalendarDate::new(2020, 1, 1)),
        PassportScope::BelowThreshold,
        "below the threshold is a category answer, not a date answer"
    );
}

/// Wire names arrive from JSON, so the match tolerates case and padding — the
/// same contract `annex_xiii_requirement` offers.
#[test]
fn wire_names_are_matched_case_insensitively_and_trimmed() {
    assert_eq!(
        passport_scope("  EV  ", None, AFTER),
        PassportScope::Required
    );
    assert_eq!(passport_scope("Lmt", None, AFTER), PassportScope::Required);
    assert_eq!(
        passport_scope(" Industrial ", Some(10.0), AFTER),
        PassportScope::Required
    );
}

/// An unrecognised type is outside a closed list of three. That is a statement
/// about the article, not a guess about the input.
#[test]
fn an_unknown_category_is_outside_a_closed_list() {
    assert_eq!(passport_scope("", None, AFTER), PassportScope::NotCovered);
    assert_eq!(
        passport_scope("flow-battery", Some(99.0), AFTER),
        PassportScope::NotCovered
    );
}

/// `is_required` is true for exactly one variant. Guards against a later variant
/// being quietly folded into "yes".
#[test]
fn only_required_reports_an_obligation() {
    for s in [
        PassportScope::NotCovered,
        PassportScope::BelowThreshold,
        PassportScope::CapacityUnknown,
        PassportScope::NotYetBinding,
    ] {
        assert!(!s.is_required(), "{s:?} must not report an obligation");
    }
    assert!(PassportScope::Required.is_required());
}
