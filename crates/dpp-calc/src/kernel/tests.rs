//! Unit tests for the methodology-agnostic spine.

use super::ruleset::{Effectivity, RulesetId};
use crate::error::CalcError;
use chrono::NaiveDate;

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

fn in_force(from: (i32, u32, u32), until: Option<(i32, u32, u32)>) -> Effectivity {
    match until {
        Some((y, m, d)) => Effectivity::closed(day(from.0, from.1, from.2), day(y, m, d)),
        None => Effectivity::open(day(from.0, from.1, from.2)),
    }
}

#[test]
fn active_on_from_date() {
    assert!(in_force((2025, 6, 1), None).is_active_on(day(2025, 6, 1)));
}

#[test]
fn inactive_before_from() {
    assert!(!in_force((2025, 6, 1), None).is_active_on(day(2025, 5, 31)));
}

#[test]
fn active_on_until_date() {
    let b = in_force((2025, 6, 1), Some((2026, 12, 31)));
    assert!(b.is_active_on(day(2026, 12, 31)));
}

#[test]
fn inactive_after_until() {
    let b = in_force((2025, 6, 1), Some((2026, 12, 31)));
    assert!(!b.is_active_on(day(2027, 1, 1)));
}

#[test]
fn open_ended_always_active_after_from() {
    assert!(in_force((2020, 1, 1), None).is_active_on(day(2099, 12, 31)));
}

#[test]
fn pending_governs_no_date_at_all() {
    // The point of the variant. A pending ruleset is not "in force from some
    // very distant day" — it has no application date, so no date resolves it,
    // however far out.
    let pending = Effectivity::pending("EU 2023/1542 Art. 10(5)", None);
    for date in [day(1900, 1, 1), day(2026, 7, 25), day(2999, 12, 31)] {
        assert!(
            !pending.is_active_on(date),
            "pending must not be active on {date}"
        );
    }
}

#[test]
fn ensure_active_on_separates_undetermined_from_not_yet_effective() {
    let id = RulesetId("test");

    // No application date exists yet — distinct from knowing the date and
    // waiting for it. Conflating the two is what the far-future sentinel did.
    let pending = Effectivity::pending("EU 2023/1542 Art. 7(2)", Some(day(2026, 8, 18)));
    assert!(matches!(
        pending.ensure_active_on(&id, day(2026, 1, 1)),
        Err(CalcError::RulesetUndetermined { .. })
    ));

    // Known start date, not yet arrived.
    let future = in_force((2031, 8, 18), None);
    assert!(matches!(
        future.ensure_active_on(&id, day(2026, 1, 1)),
        Err(CalcError::RulesetNotYetEffective { .. })
    ));

    // After `until` → expired.
    let past = in_force((2020, 1, 1), Some((2021, 1, 1)));
    assert!(matches!(
        past.ensure_active_on(&id, day(2026, 1, 1)),
        Err(CalcError::RulesetExpired { .. })
    ));

    // Within the period → ok.
    assert!(past.ensure_active_on(&id, day(2020, 6, 1)).is_ok());
}

#[test]
fn undetermined_error_names_the_empowerment_being_waited_on() {
    let id = RulesetId("laptop-repairability");
    let pending = Effectivity::pending("ESPR (EU) 2024/1781 — laptop delegated act", None);
    let err = pending
        .ensure_active_on(&id, day(2026, 7, 25))
        .expect_err("pending must not resolve");
    let msg = err.to_string();
    assert!(
        msg.contains("laptop-repairability") && msg.contains("ESPR (EU) 2024/1781"),
        "error must say which ruleset and which instrument: {msg}"
    );
}
