//! Unit tests for the methodology-agnostic spine.

use super::ruleset::EffectiveDateBound;
use chrono::NaiveDate;

fn make(from: (i32, u32, u32), until: Option<(i32, u32, u32)>) -> EffectiveDateBound {
    EffectiveDateBound {
        from: NaiveDate::from_ymd_opt(from.0, from.1, from.2).unwrap(),
        until: until.map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap()),
    }
}

#[test]
fn active_on_from_date() {
    let b = make((2025, 6, 1), None);
    assert!(b.is_active_on(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()));
}

#[test]
fn inactive_before_from() {
    let b = make((2025, 6, 1), None);
    assert!(!b.is_active_on(NaiveDate::from_ymd_opt(2025, 5, 31).unwrap()));
}

#[test]
fn active_on_until_date() {
    let b = make((2025, 6, 1), Some((2026, 12, 31)));
    assert!(b.is_active_on(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()));
}

#[test]
fn inactive_after_until() {
    let b = make((2025, 6, 1), Some((2026, 12, 31)));
    assert!(!b.is_active_on(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()));
}

#[test]
fn open_ended_always_active_after_from() {
    let b = make((2020, 1, 1), None);
    assert!(b.is_active_on(NaiveDate::from_ymd_opt(2099, 12, 31).unwrap()));
}
