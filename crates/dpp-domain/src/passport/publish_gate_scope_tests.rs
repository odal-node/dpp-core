//! The publish gate's questions before content: whether Art. 77(1) reaches the
//! record at all, and the facts an obligation turns on (the placing date and
//! Art. 47).

use super::publish_gate_tests::{MARCH_2027, battery_field, publishable_battery, refusal_paths};
use super::*;
use crate::status::PassportStatus;

// ── Art. 77(1) scope: is a passport owed before asking what it must contain ──
//
// The content table is keyed on category alone, so on its own it holds every
// industrial battery to the full industrial list. Art. 77(1) reaches industrial
// batteries with "a capacity greater than 2 kWh", placed on the market from
// 18 February 2027. These cover both directions of each boundary, because a gate
// that exempts too much is the failure that does not announce itself.

/// Build an industrial battery missing one field the category makes mandatory,
/// so the only question left is whether Art. 77(1) reaches the record.
fn deficient_industrial(capacity_kwh: Option<f64>) -> Passport {
    let mut p = publishable_battery(crate::product_group::BatteryType::Industrial);
    battery_field(&mut p, |b| {
        b.rated_capacity_kwh = capacity_kwh;
        b.usable_extinguishing_agent = None;
    });
    p
}

#[test]
fn an_industrial_battery_at_or_below_two_kwh_is_not_held_to_industrial_content() {
    // "greater than 2 kWh" is strict, so 2,0 kWh itself owes nothing.
    for kwh in [1.0, 2.0] {
        let mut p = deficient_industrial(Some(kwh));
        p.placed_on_market_date = Some(chrono::NaiveDate::from_ymd_opt(2030, 1, 1).unwrap());
        p.transition_to(PassportStatus::Published)
            .unwrap_or_else(|e| {
                panic!("Art. 77(1) does not reach a {kwh} kWh industrial battery: {e}")
            });
    }
}

#[test]
fn an_industrial_battery_above_two_kwh_is_still_gated() {
    let mut p = deficient_industrial(Some(2.000_001));
    p.placed_on_market_date = Some(chrono::NaiveDate::from_ymd_opt(2030, 1, 1).unwrap());
    let err = p
        .transition_to(PassportStatus::Published)
        .expect_err("just over the threshold is over it");
    assert!(
        err.to_string().contains("usableExtinguishingAgent"),
        "{err}"
    );
}

#[test]
fn an_industrial_battery_that_states_no_capacity_is_still_gated() {
    // The one that matters most. `CapacityUnknown` is not an exemption: the
    // obligation turns on a number the record does not carry, and exempting on
    // a missing field is how a statutory gate switches itself off in silence.
    let mut p = deficient_industrial(None);
    p.placed_on_market_date = Some(chrono::NaiveDate::from_ymd_opt(2030, 1, 1).unwrap());
    let err = p
        .transition_to(PassportStatus::Published)
        .expect_err("an unstated capacity must not exempt");
    assert!(
        err.to_string().contains("usableExtinguishingAgent"),
        "{err}"
    );
}

#[test]
fn the_eighteenth_of_february_2027_is_inside_the_obligation_and_the_day_before_is_not() {
    for (date, gated) in [
        (chrono::NaiveDate::from_ymd_opt(2027, 2, 17).unwrap(), false),
        (chrono::NaiveDate::from_ymd_opt(2027, 2, 18).unwrap(), true),
    ] {
        let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
        battery_field(&mut p, |b| b.usable_extinguishing_agent = None);
        p.placed_on_market_date = Some(date);
        let outcome = p.transition_to(PassportStatus::Published);
        assert_eq!(
            outcome.is_err(),
            gated,
            "Art. 77(1) applies from 18 February 2027; {date} answered {outcome:?}"
        );
    }
}

#[test]
fn a_record_stating_no_placing_date_is_still_gated() {
    // A draft for a product not yet on the market carries no date. Reading that
    // as "before 2027" would exempt every draft, which is every passport at the
    // moment the gate runs.
    let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);
    p.placed_on_market_date = None;
    let err = p
        .transition_to(PassportStatus::Published)
        .expect_err("an unstated placing date must not exempt");
    assert!(
        err.to_string().contains("usableExtinguishingAgent"),
        "{err}"
    );
}

// ── Facts an obligation turns on: the placing date and Art. 47 ──────────────
//
// Whether `dueDiligenceUrl` is owed turns on the placing date (Art. 48(1), from
// 18 August 2027) and on Art. 47's two exemptions. A missing fact is refused by
// one rule, once per fact, naming what waits on it.

const DUE_DILIGENCE_FROM: chrono::NaiveDate = chrono::NaiveDate::from_ymd_opt(2027, 8, 18).unwrap();

#[test]
fn a_complete_battery_without_a_placing_date_is_refused_for_that_alone() {
    for battery_type in [
        crate::product_group::BatteryType::Ev,
        crate::product_group::BatteryType::Lmt,
        crate::product_group::BatteryType::Industrial,
    ] {
        let mut p = publishable_battery(battery_type.clone());
        battery_field(&mut p, |b| b.rated_capacity_kwh = Some(60.0));
        p.placed_on_market_date = None;
        assert_eq!(
            refusal_paths(&p),
            ["/placedOnMarketDate"],
            "{battery_type:?}: one error, for the one missing fact"
        );
        let err = p.transition_to(PassportStatus::Published).unwrap_err();
        assert!(err.to_string().contains("'dueDiligenceUrl'"), "{err}");
        assert_eq!(p.status, PassportStatus::Draft);
    }
}

#[test]
fn the_battery_s_own_placing_date_is_enough() {
    let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
    p.placed_on_market_date = None;
    battery_field(&mut p, |b| b.placed_on_market_date = Some(MARCH_2027));
    p.transition_to(PassportStatus::Published)
        .expect("either home of the date states it");
}

#[test]
fn the_day_before_18_august_2027_owes_no_report_and_no_declaration() {
    let mut p = publishable_battery(crate::product_group::BatteryType::Lmt);
    p.placed_on_market_date = DUE_DILIGENCE_FROM.pred_opt();
    p.transition_to(PassportStatus::Published)
        .expect("Art. 48(1) does not reach a battery placed before it applied");
}

#[test]
fn from_18_august_2027_the_report_is_owed_unless_art_47_exempts() {
    use crate::product_group::DueDiligenceOperatorScope as Scope;

    let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
    p.placed_on_market_date = Some(DUE_DILIGENCE_FROM);
    assert_eq!(
        refusal_paths(&p),
        ["/productGroupData/dueDiligenceOperatorScope"],
        "an undeclared operator is asked, not excused"
    );

    battery_field(&mut p, |b| {
        b.due_diligence_operator_scope = Some(Scope::Subject)
    });
    assert_eq!(refusal_paths(&p), ["/productGroupData/dueDiligenceUrl"]);

    battery_field(&mut p, |b| {
        b.due_diligence_operator_scope = Some(Scope::BelowTurnoverThreshold);
    });
    assert_eq!(
        refusal_paths(&p),
        Vec::<String>::new(),
        "Art. 47, first paragraph"
    );

    battery_field(&mut p, |b| {
        b.due_diligence_operator_scope = Some(Scope::Subject);
        b.due_diligence_url = Some("https://example.invalid/due-diligence".into());
    });
    p.transition_to(PassportStatus::Published)
        .expect("a declared subject carrying the report's address publishes");
}

/// Art. 77(7) gives a second-life battery a new passport with a later placing
/// date. Art. 47's second paragraph takes it out of the chapter at any date, so
/// it is asked for neither the date nor the declaration on this row's account.
#[test]
fn a_second_life_battery_needs_no_date_and_no_declaration() {
    use crate::product_group::BatteryStatus;
    for status in [
        BatteryStatus::Repurposed,
        BatteryStatus::ReUsed,
        BatteryStatus::Remanufactured,
    ] {
        let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
        battery_field(&mut p, |b| b.battery_status = Some(status));
        p.placed_on_market_date = None;
        assert_eq!(refusal_paths(&p), Vec::<String>::new(), "{status:?}");
    }

    // `waste` is not a second-life operation, so it gets no exemption.
    let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.battery_status = Some(BatteryStatus::Waste));
    p.placed_on_market_date = None;
    assert_eq!(refusal_paths(&p), ["/placedOnMarketDate"]);
}

#[test]
fn placing_dates_that_disagree_are_refused_by_the_gate_itself() {
    // `transition_to` does not call `validate`, so the gate says it too.
    let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| {
        b.placed_on_market_date = Some(DUE_DILIGENCE_FROM)
    });
    assert_eq!(refusal_paths(&p), ["/productGroupData/placedOnMarketDate"]);
}

#[test]
fn the_strict_check_still_answers_for_a_record_art_77_1_exempts() {
    // A node holding a voluntary passport to its category's content has to be
    // able to ask, or this change removes a capability instead of correcting
    // one. The same record publishes and fails the strict check.
    let mut p = deficient_industrial(Some(1.0));
    p.placed_on_market_date = Some(chrono::NaiveDate::from_ymd_opt(2030, 1, 1).unwrap());

    let err = p
        .check_category_content()
        .expect_err("the strict check ignores scope, and the field is still absent");
    assert!(
        err.to_string().contains("usableExtinguishingAgent"),
        "{err}"
    );

    p.transition_to(PassportStatus::Published)
        .expect("and the publish gate still lets it through");
}

/// 🚨 Two placing dates that disagree must not buy an exemption.
///
/// `validate` refuses a record whose envelope and battery dates differ — but
/// `transition_to` never calls it, going straight to `check_mandatory_content`.
/// The exemption helper read `envelope.or(battery)`, so the envelope's date won
/// outright: a pre-2027 envelope date beside a 2030 battery date answered
/// `NotYetBinding`, and the content gate a published EV battery has to pass was
/// skipped. An exemption reached by the record contradicting itself.
///
/// Fail closed: disagreement is not an exemption, in either direction.
#[test]
fn placing_dates_that_disagree_do_not_exempt_the_content_gate() {
    let before = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let after = chrono::NaiveDate::from_ymd_opt(2030, 1, 1).unwrap();

    for (envelope, product_group) in [(before, after), (after, before)] {
        let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
        battery_field(&mut p, |b| b.usable_extinguishing_agent = None);
        battery_field(&mut p, |b| b.placed_on_market_date = Some(product_group));
        p.placed_on_market_date = Some(envelope);

        assert!(
            p.transition_to(PassportStatus::Published).is_err(),
            "envelope {envelope} against product-group {product_group} was \
             exempted from the content gate — a record disagreeing with itself \
             must not publish incomplete content"
        );
    }
}

/// The other half, so the fix above is not simply "always gate": when the two
/// dates agree and sit before 18 February 2027, the exemption still applies.
#[test]
fn placing_dates_that_agree_before_the_date_still_exempt() {
    let before = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let mut p = publishable_battery(crate::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);
    battery_field(&mut p, |b| b.placed_on_market_date = Some(before));
    p.placed_on_market_date = Some(before);

    p.transition_to(PassportStatus::Published)
        .expect("Art. 77(1) does not reach a battery placed before 18 February 2027");
}
