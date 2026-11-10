//! How long the documents behind a discard are kept.

use chrono::NaiveDate;

use crate::test_support::sample_discard_record;
use crate::unsold_goods::{DiscardRecord, Disclosure, EnterpriseSize, KeepUntil, keep_until};

/// A calendar date, for readability.
fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

/// Del. Reg. (EU) 2026/296 Art. 3 runs five years from the destruction, Impl. Reg.
/// (EU) 2026/2 Art. 4 five years from the disclosure; the documents are held for the
/// later, which is the disclosure's whenever it is made after the discard.
#[test]
fn keep_until_is_the_later_of_destruction_and_disclosure_plus_five_years() {
    let discarded = date(2027, 3, 14);

    assert_eq!(
        keep_until(discarded, Disclosure::MadeOn(date(2028, 9, 1))),
        KeepUntil::On(date(2033, 9, 1))
    );
    // A disclosure dated before the discard is a mistake in the data; the answer
    // must still never be earlier than the destruction's five years.
    assert_eq!(
        keep_until(date(2030, 1, 1), Disclosure::MadeOn(date(2028, 1, 1))),
        KeepUntil::On(date(2035, 1, 1))
    );
}

/// Until the covering disclosure is made, the answer is open.
#[test]
fn keep_until_is_open_before_the_disclosure_exists() {
    assert_eq!(
        keep_until(date(2027, 3, 14), Disclosure::Pending),
        KeepUntil::Open {
            not_before: date(2032, 3, 14)
        }
    );
}

/// An operator may disclose up to twelve months after the year ends, or later. Until
/// it has, the five years after the disclosure have not begun.
#[test]
fn an_open_answer_never_permits_deletion() {
    let open = keep_until(date(2027, 3, 14), Disclosure::Pending);
    assert!(!open.permits_deletion_on(date(2100, 1, 1)));
}

/// Documents may be deleted after the last day, not on it.
#[test]
fn a_fixed_answer_permits_deletion_only_after_its_last_day() {
    let fixed = KeepUntil::On(date(2033, 9, 1));
    assert!(!fixed.permits_deletion_on(date(2033, 8, 31)));
    assert!(!fixed.permits_deletion_on(date(2033, 9, 1)));
    assert!(fixed.permits_deletion_on(date(2033, 9, 2)));
}

/// 2033 is not a leap year, so five years after 29 February 2028 falls on the last
/// day of that month, the way `FinancialYear::disclosure_due_by` adds its twelve.
#[test]
fn five_years_after_29_february_lands_on_the_last_day_of_february() {
    assert_eq!(
        keep_until(date(2028, 2, 29), Disclosure::MadeOn(date(2028, 2, 29))),
        KeepUntil::On(date(2033, 2, 28))
    );
}

/// A date the calendar cannot add five years to is kept to the end of it, never
/// released early.
#[test]
fn a_date_too_far_out_to_add_five_years_to_is_kept_to_the_end_of_the_calendar() {
    assert_eq!(
        keep_until(NaiveDate::MAX, Disclosure::MadeOn(NaiveDate::MAX)),
        KeepUntil::On(NaiveDate::MAX)
    );
}

/// An operator that owes no disclosure has no second period to wait for, so the
/// answer is a date and the documents can go once it passes. It used to be `Open`
/// for ever, which is a retention duty the acts do not impose.
#[test]
fn a_discard_no_disclosure_covers_is_kept_five_years_from_the_discard() {
    let discarded = date(2027, 3, 14);
    let answer = keep_until(discarded, Disclosure::NotOwed);

    assert_eq!(answer, KeepUntil::On(date(2032, 3, 14)));
    assert!(answer.permits_deletion_on(date(2032, 3, 15)));
}

/// The state follows from who the operator is: an exempt one owes none and a
/// disclosure it made anyway starts nothing, one that is reached owes one that is
/// pending until it is made.
#[test]
fn the_disclosure_state_follows_from_the_operators_size() {
    let discarded = date(2027, 3, 14);
    let made = Some(date(2028, 9, 1));

    assert_eq!(
        Disclosure::of(EnterpriseSize::Micro, discarded, made),
        Disclosure::NotOwed
    );
    assert_eq!(
        Disclosure::of(EnterpriseSize::Small, discarded, None),
        Disclosure::NotOwed
    );
    assert_eq!(
        Disclosure::of(EnterpriseSize::Medium, discarded, made),
        Disclosure::NotOwed,
        "a medium-sized operator is outside the duty until 19 July 2030"
    );
    assert_eq!(
        Disclosure::of(EnterpriseSize::Medium, date(2030, 7, 19), None),
        Disclosure::Pending
    );
    assert_eq!(
        Disclosure::of(EnterpriseSize::Large, discarded, None),
        Disclosure::Pending
    );
    assert_eq!(
        Disclosure::of(EnterpriseSize::Large, discarded, made),
        Disclosure::MadeOn(date(2028, 9, 1))
    );
}

/// The record's own answer takes the operator's size, so a host cannot ask the
/// question of an exempt operator and be told to wait for a disclosure.
#[test]
fn an_exempt_operators_record_is_not_held_open() {
    let record = DiscardRecord {
        discarded_on: date(2027, 3, 14),
        ..sample_discard_record()
    };
    assert_eq!(
        record.keep_until(EnterpriseSize::Micro, None),
        KeepUntil::On(date(2032, 3, 14))
    );
    assert_eq!(
        record.keep_until(EnterpriseSize::Large, None),
        KeepUntil::Open {
            not_before: date(2032, 3, 14)
        }
    );
}

/// A record's answer runs from the day it was discarded.
#[test]
fn a_record_keeps_until_from_the_day_it_was_discarded() {
    let record = DiscardRecord {
        discarded_on: date(2027, 3, 14),
        ..sample_discard_record()
    };
    assert_eq!(
        record.keep_until(EnterpriseSize::Large, Some(date(2028, 9, 1))),
        KeepUntil::On(date(2033, 9, 1))
    );
}
