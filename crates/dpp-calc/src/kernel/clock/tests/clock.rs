//! Tests of the `clock` module.

use super::super::*;
use chrono::{DateTime, NaiveDate, Utc};

#[test]
fn placed_on_takes_the_law_date_from_the_caller_not_the_clock() {
    let placed = NaiveDate::from_ymd_opt(2030, 6, 1).unwrap();
    let clock = AssessmentClock::placed_on(placed);
    assert_eq!(clock.law_in_force_on, placed);
    // The wall clock only ever supplies `computed_at`.
    assert!(clock.computed_at > DateTime::UNIX_EPOCH);
    assert_ne!(clock.law_in_force_on, clock.computed_at.date_naive());
}

#[test]
fn new_pins_both_dates_for_receipt_replay() {
    let law = NaiveDate::from_ymd_opt(2031, 8, 18).unwrap();
    let computed = DateTime::parse_from_rfc3339("2033-04-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let clock = AssessmentClock::new(law, computed);
    assert_eq!(clock.law_in_force_on, law);
    assert_eq!(clock.computed_at, computed);
}
