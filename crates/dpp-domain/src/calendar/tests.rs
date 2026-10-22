//! The obligation calendar: what it holds, its order, its window, and the basis
//! every event carries.

use chrono::NaiveDate;

use super::obligation_calendar::RULE_SCHEDULES;
use super::*;
use crate::instrument::currency::CurrencyState;
use crate::instrument::{
    DateBasis, InstrumentCatalog, InstrumentStatus, ObligationDate, PassportObligation,
};

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn ids(events: &[ObligationEvent]) -> Vec<&str> {
    events.iter().map(|e| e.id.as_str()).collect()
}

/// Every sourced date the repository holds today, in order. A new one fails
/// this test until it is added here, which is the point: the list is read by a
/// person before it is announced to anyone.
#[test]
fn the_built_in_calendar_holds_every_sourced_date_in_order() {
    let calendar = ObligationCalendar::new();
    let got: Vec<(NaiveDate, &str)> = calendar
        .events()
        .iter()
        .map(|e| (e.date, e.id.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            (day(2024, 8, 18), "battery.lead.portable"),
            (day(2027, 2, 18), "battery-reg-2023-1542.passport.battery"),
            (day(2027, 8, 18), "battery.due-diligence-report"),
            (day(2028, 8, 18), "battery.lead.zinc-air-button-cells"),
            (day(2029, 9, 23), "detergents-2026-405.passport.detergent"),
            (day(2030, 8, 1), "toy-safety-2025-2509.passport.toy"),
            (
                day(2031, 8, 18),
                "battery.recycled-content.minimum-shares-2031"
            ),
            (day(2032, 9, 1), "elv-2026-1738.passport"),
            (
                day(2036, 8, 18),
                "battery.recycled-content.minimum-shares-2036"
            ),
        ]
    );
    assert!(
        calendar
            .events()
            .iter()
            .all(|e| e.basis == DateBasis::Sourced),
        "every date the embedded sources hold is sourced today"
    );
}

#[test]
fn every_rule_change_becomes_an_event() {
    // `rule_event` drops a date it cannot place, so a malformed constant would
    // otherwise disappear without a word.
    let scheduled: usize = RULE_SCHEDULES.iter().map(|s| s.len()).sum();
    let calendar = ObligationCalendar::new();
    let from_rules = calendar
        .events()
        .iter()
        .filter(|e| e.kind != ObligationKind::PassportStart)
        .count();
    assert_eq!(from_rules, scheduled);
}

#[test]
fn ids_are_unique() {
    let calendar = ObligationCalendar::new();
    let mut seen = ids(calendar.events());
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), calendar.events().len());
}

#[test]
fn the_window_includes_both_ends_and_a_reversed_one_is_empty() {
    let calendar = ObligationCalendar::new();
    let on = |from, to| {
        calendar
            .between(from, to)
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>()
    };
    let date = day(2027, 8, 18);
    assert_eq!(on(date, date), ["battery.due-diligence-report"]);
    assert_eq!(
        on(day(2027, 2, 18), day(2027, 8, 18)),
        [
            "battery-reg-2023-1542.passport.battery",
            "battery.due-diligence-report"
        ]
    );
    assert!(on(day(2027, 8, 19), day(2028, 8, 17)).is_empty());
    assert!(on(date, day(2027, 8, 17)).is_empty(), "from after to");
}

/// The due diligence report date, with its citation, reaching the three
/// categories that carry a passport.
#[test]
fn the_due_diligence_report_is_on_the_calendar() {
    let calendar = ObligationCalendar::new();
    let event = calendar
        .events()
        .iter()
        .find(|e| e.id == "battery.due-diligence-report")
        .unwrap();
    assert_eq!(event.date, day(2027, 8, 18));
    assert_eq!(event.kind, ObligationKind::ContentRequirement);
    assert!(event.citation.contains("Art. 48(1)"), "{}", event.citation);
    assert_eq!(event.categories, ["ev", "lmt", "industrial"]);
    assert_eq!(event.product_group.as_deref(), Some("battery"));
}

/// The Art. 77(1) constant is not on the calendar in its own right, because the
/// battery instrument already records the same date. So the two must agree.
#[test]
fn the_art_77_1_constant_agrees_with_the_battery_instrument() {
    let required_from = dpp_rules::batteries::passport_scope::PASSPORT_REQUIRED_FROM;
    let calendar = ObligationCalendar::new();
    let event = calendar
        .events()
        .iter()
        .find(|e| e.id == "battery-reg-2023-1542.passport.battery")
        .unwrap();
    assert_eq!(
        event.date,
        day(
            required_from.year,
            u32::from(required_from.month),
            u32::from(required_from.day)
        )
    );
}

/// An assumed date is returned, and says so. It is the consumer's job not to
/// present it as a deadline, and it can only do that if the event carries the
/// basis.
#[test]
fn an_assumed_date_carries_its_basis() {
    let mut catalog = InstrumentCatalog::new();
    let mut assumed = catalog.get("toy-safety-2025-2509").unwrap().clone();
    assumed.id = "an-assumed-act".to_owned();
    assumed.passport = PassportObligation::Required {
        from: Some(ObligationDate {
            date: "2031-01-01".to_owned(),
            basis: DateBasis::Assumed,
        }),
    };
    for binding in &mut assumed.product_groups {
        binding.passport = None;
    }
    catalog.register(assumed).unwrap();

    let calendar = ObligationCalendar::from_catalog(&catalog);
    let event = calendar
        .events()
        .iter()
        .find(|e| e.id == "an-assumed-act.passport.toy")
        .unwrap();
    assert_eq!(event.date, day(2031, 1, 1));
    assert_eq!(event.basis, DateBasis::Assumed);
}

/// A dated passport obligation is announced only from an act that is current
/// law. A proposal, a repealed act and an adopted act whose currency was never
/// checked each keep the toy act's sourced date, and none of them reaches the
/// calendar; the toy act itself, beside them, still does.
#[test]
fn an_act_that_is_not_current_law_announces_nothing() {
    let mut catalog = InstrumentCatalog::new();
    let toy = catalog.get("toy-safety-2025-2509").unwrap().clone();

    let mut proposed = toy.clone();
    proposed.id = "a-proposed-act".to_owned();
    proposed.status = InstrumentStatus::Proposed;

    let mut repealed = toy.clone();
    repealed.id = "a-repealed-act".to_owned();
    repealed.currency.as_mut().unwrap().state = CurrencyState::Repealed {
        by: "32099R0001".to_owned(),
        on: "2029-01-01".to_owned(),
    };

    let mut unchecked = toy;
    unchecked.id = "an-unchecked-act".to_owned();
    unchecked.currency = None;

    for act in [proposed, repealed, unchecked] {
        catalog.register(act).unwrap();
    }

    let calendar = ObligationCalendar::from_catalog(&catalog);
    let on_calendar = ids(calendar.events());
    for id in ["a-proposed-act", "a-repealed-act", "an-unchecked-act"] {
        assert!(
            !on_calendar.iter().any(|e| e.starts_with(id)),
            "{id} is not current law and must announce nothing: {on_calendar:?}"
        );
    }
    assert!(
        on_calendar.contains(&"toy-safety-2025-2509.passport.toy"),
        "{on_calendar:?}"
    );
}

#[test]
fn an_undated_passport_obligation_is_not_an_event() {
    // ESPR requires a passport for six product groups and dates none of them.
    let calendar = ObligationCalendar::new();
    assert!(
        !calendar.events().iter().any(|e| e.id.starts_with("espr.")),
        "{:?}",
        ids(calendar.events())
    );
}

#[test]
fn an_event_serialises_with_its_basis_and_a_plain_date() {
    let calendar = ObligationCalendar::new();
    let event = calendar
        .events()
        .iter()
        .find(|e| e.id == "elv-2026-1738.passport")
        .unwrap();
    let wire = serde_json::to_value(event).unwrap();
    assert_eq!(wire["date"], "2032-09-01");
    assert_eq!(wire["kind"], "passportStart");
    assert_eq!(wire["basis"], "sourced");
    // No product group is modelled for vehicles, and the event says nothing
    // rather than inventing one.
    assert!(wire.get("productGroup").is_none());
    assert!(wire.get("categories").is_none());
}
