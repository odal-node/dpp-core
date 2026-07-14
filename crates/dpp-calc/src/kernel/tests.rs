//! Unit tests for the methodology-agnostic spine.

use super::ruleset::{EffectiveDateBound, RulesetId};
use crate::error::CalcError;
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

#[test]
fn ensure_active_on_distinguishes_not_yet_effective_from_expired() {
    let id = RulesetId("test".into());
    let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();

    // Before `from` → not yet effective (not "expired").
    let future = make((2100, 1, 1), None);
    assert!(matches!(
        future.ensure_active_on(&id, day(2026, 1, 1)),
        Err(CalcError::RulesetNotYetEffective { .. })
    ));

    // After `until` → expired.
    let past = make((2020, 1, 1), Some((2021, 1, 1)));
    assert!(matches!(
        past.ensure_active_on(&id, day(2026, 1, 1)),
        Err(CalcError::RulesetExpired { .. })
    ));

    // Within the period → ok.
    assert!(past.ensure_active_on(&id, day(2020, 6, 1)).is_ok());
}
