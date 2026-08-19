//! Regression vectors for the Art. 8 recycled-content determination.

use chrono::NaiveDate;
use dpp_rules::batteries::recycled_content as rules;

use super::*;
use crate::assessability::Assessability;
use crate::clock::AssessmentClock;
use crate::error::CalcError;
use crate::ruleset::Ruleset;
use crate::ruleset_registry::resolve_recycled_content;

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

fn clock(y: i32, m: u32, d: u32) -> AssessmentClock {
    AssessmentClock::placed_on(day(y, m, d))
}

/// Declared shares that clear Art. 8(2) but not Art. 8(3).
///
/// Cobalt 16 % is exactly the 2031 minimum and below the 2036 one (26 %);
/// lithium 6 % likewise (12 % from 2036). The same battery is compliant under
/// one phase and short under the next, which is the whole reason the phase is
/// selected by date rather than assumed.
fn between_the_phases() -> RecycledContentInputs {
    RecycledContentInputs {
        cobalt_pct: Some(16.0),
        lithium_pct: Some(6.0),
        nickel_pct: Some(6.0),
        lead_pct: Some(85.0),
    }
}

#[test]
fn meeting_the_minimum_exactly_is_not_a_shortfall() {
    let result = calculate(
        &between_the_phases(),
        &Art8Phase1Ruleset,
        clock(2031, 8, 18),
    )
    .expect("phase 1 governs a battery placed on its first day");
    assert!(
        result.shortfalls.is_empty(),
        "a share equal to the minimum meets it: {:?}",
        result.shortfalls
    );
}

#[test]
fn the_same_shares_fall_short_under_phase_two() {
    let result = calculate(
        &between_the_phases(),
        &Art8Phase2Ruleset,
        clock(2036, 8, 18),
    )
    .expect("phase 2 governs a battery placed on its first day");
    let mut short: Vec<&str> = result.shortfalls.iter().map(|s| s.metal.as_str()).collect();
    short.sort_unstable();
    assert_eq!(
        short,
        vec!["cobalt", "lithium", "nickel"],
        "lead is 85 % in both phases and must not be reported"
    );
    let cobalt = result
        .shortfalls
        .iter()
        .find(|s| s.metal == "cobalt")
        .expect("cobalt is short");
    assert_eq!(cobalt.declared_pct, 16.0);
    assert_eq!(cobalt.required_pct, rules::COBALT_RECYCLED_PCT_2036);
}

/// An undeclared share is not a zero.
///
/// Art. 8 sets a minimum for a share that was declared. Treating absence as 0 %
/// would report every battery as failing every metal it did not mention.
#[test]
fn undeclared_metals_are_not_shortfalls() {
    let result = calculate(
        &RecycledContentInputs::default(),
        &Art8Phase2Ruleset,
        clock(2036, 8, 18),
    )
    .expect("valid");
    assert!(result.shortfalls.is_empty());
}

/// Being outside a phase is not a shortfall — it is an error the caller must
/// distinguish, because it means the rule does not apply.
#[test]
fn a_battery_placed_before_the_phase_is_refused_not_failed() {
    let err = calculate(&between_the_phases(), &Art8Phase1Ruleset, clock(2030, 1, 1))
        .expect_err("Art. 8(2) does not reach a battery placed on the market in 2030");
    assert!(
        matches!(err, CalcError::RulesetNotYetEffective { .. }),
        "got {err}"
    );
}

/// Phase 1 closes the day Phase 2 opens, so exactly one governs any date.
#[test]
fn the_two_phases_never_both_govern_a_date() {
    for d in [
        day(2031, 8, 18),
        day(2033, 6, 1),
        day(2036, 8, 17),
        day(2036, 8, 18),
        day(2040, 1, 1),
    ] {
        let active = [
            Art8Phase1Ruleset.effectivity().is_active_on(d),
            Art8Phase2Ruleset.effectivity().is_active_on(d),
        ];
        assert_eq!(
            active.iter().filter(|a| **a).count(),
            1,
            "exactly one phase must govern {d}, got {active:?}"
        );
    }
}

// ── Resolution ───────────────────────────────────────────────────────────────

#[test]
fn industrial_ev_sli_resolves_to_the_phase_its_market_date_falls_in() {
    let phase1 = resolve_recycled_content("industrial-ev-sli", day(2032, 1, 1))
        .assessed()
        .expect("phase 1 governs 2032");
    assert_eq!(phase1.id().0, "battery-recycled-content-art8-2");

    let phase2 = resolve_recycled_content("industrial-ev-sli", day(2037, 1, 1))
        .assessed()
        .expect("phase 2 governs 2037");
    assert_eq!(phase2.id().0, "battery-recycled-content-art8-3");
}

/// The scope difference that is not a date.
///
/// Art. 8(2) never names LMT batteries, so one placed on the market in 2032 is
/// not short of anything — it is waiting for Art. 8(3). Resolving it to Phase 1
/// with an empty shortfall list would report a rule as satisfied that never
/// applied to it.
#[test]
fn an_lmt_battery_before_2036_is_not_yet_in_force_rather_than_compliant() {
    match resolve_recycled_content("lmt", day(2032, 1, 1)) {
        Assessability::NotYetInForce {
            ruleset_id,
            applies_from,
        } => {
            assert_eq!(ruleset_id, "battery-recycled-content-art8-3");
            assert_eq!(applies_from, day(2036, 8, 18));
        }
        _ => panic!("expected NotYetInForce for an LMT battery placed in 2032"),
    }
    assert!(
        resolve_recycled_content("lmt", day(2036, 8, 18)).is_assessed(),
        "Art. 8(3) reaches LMT from its first day"
    );
}

/// A portable battery is outside Art. 8 entirely, which is not non-compliance.
#[test]
fn a_category_art8_does_not_reach_is_out_of_scope() {
    assert!(matches!(
        resolve_recycled_content("portable", day(2040, 1, 1)),
        Assessability::OutOfScope
    ));
}

/// Before any phase binds, the answer names the soonest date, not the furthest.
#[test]
fn a_pre_2031_industrial_battery_is_told_about_phase_one() {
    match resolve_recycled_content("industrial-ev-sli", day(2026, 1, 1)) {
        Assessability::NotYetInForce { applies_from, .. } => {
            assert_eq!(applies_from, day(2031, 8, 18));
        }
        _ => panic!("expected NotYetInForce naming 2031"),
    }
}

// ── Guards ───────────────────────────────────────────────────────────────────

/// This crate must never answer differently from `dpp-rules`.
///
/// The thresholds and the comparison live there; this module only selects a
/// phase and wraps the answer in a receipt. If the two ever disagree, the
/// finding an operator sees depends on whether it reached them through a Wasm
/// plugin or through here, and nothing else in either crate would notice.
#[test]
fn the_determination_agrees_with_dpp_rules_for_both_phases() {
    let shares = [
        None,
        Some(0.0),
        Some(5.9),
        Some(6.0),
        Some(50.0),
        Some(100.0),
    ];
    for cobalt in shares {
        for lithium in shares {
            for lead in [None, Some(84.9), Some(85.0)] {
                let inputs = RecycledContentInputs {
                    cobalt_pct: cobalt,
                    lithium_pct: lithium,
                    nickel_pct: Some(6.0),
                    lead_pct: lead,
                };
                let rules_input = rules::RecycledContentInput::from(&inputs);

                for (ruleset, expected) in [
                    (
                        &Art8Phase1Ruleset as &dyn RecycledContentRuleset,
                        rules::art8_shortfalls_2031(&rules_input),
                    ),
                    (
                        &Art8Phase2Ruleset as &dyn RecycledContentRuleset,
                        rules::art8_shortfalls_2036(&rules_input),
                    ),
                ] {
                    let ours = ruleset.shortfalls(&inputs);
                    assert_eq!(
                        ours.len(),
                        expected.len(),
                        "{} disagreed on {inputs:?}",
                        ruleset.id().0
                    );
                    for (a, b) in ours.iter().zip(expected.iter()) {
                        assert_eq!(a.metal, b.material);
                        assert_eq!(a.required_pct, b.required_pct);
                        assert_eq!(a.declared_pct, b.declared_pct);
                    }
                }
            }
        }
    }
}

/// A share outside 0–100 is uninterpretable, not conservatively non-compliant.
#[test]
fn an_impossible_share_is_refused() {
    for bad in [-1.0, 100.1, f64::NAN, f64::INFINITY] {
        let inputs = RecycledContentInputs {
            cobalt_pct: Some(bad),
            ..RecycledContentInputs::default()
        };
        assert!(
            matches!(
                calculate(&inputs, &Art8Phase2Ruleset, clock(2036, 8, 18)),
                Err(CalcError::InvalidInput(_))
            ),
            "{bad} must be refused"
        );
    }
}

/// The receipt cites the ruleset that ran and the date its law was read on.
#[test]
fn the_receipt_names_the_phase_and_the_market_date() {
    let placed = day(2036, 8, 18);
    let result = calculate(
        &between_the_phases(),
        &Art8Phase2Ruleset,
        AssessmentClock::placed_on(placed),
    )
    .expect("valid");
    assert_eq!(result.receipt.ruleset_id, "battery-recycled-content-art8-3");
    assert_eq!(result.receipt.ruleset_version, "1.0.0");
    assert_eq!(result.receipt.assessed_as_of, placed);
    assert!(!result.receipt.input_hash.is_empty());
    assert!(!result.receipt.output_hash.is_empty());
    // No factor dataset is involved in a threshold comparison.
    assert!(result.receipt.factor_set_hash.is_none());
}

/// Identical inputs and ruleset reproduce identical hashes — the property a
/// notified body re-running the determination relies on.
#[test]
fn the_same_inputs_hash_the_same_way() {
    let a = calculate(
        &between_the_phases(),
        &Art8Phase2Ruleset,
        clock(2036, 8, 18),
    )
    .unwrap();
    let b = calculate(
        &between_the_phases(),
        &Art8Phase2Ruleset,
        clock(2036, 8, 18),
    )
    .unwrap();
    assert_eq!(a.receipt.input_hash, b.receipt.input_hash);
    assert_eq!(a.receipt.output_hash, b.receipt.output_hash);
}

/// Both phases carry a citation. The CI sweep in `ruleset_registry` covers this
/// for every ruleset; this states it locally so a new phase added here fails
/// beside its own definition.
#[test]
fn both_phases_cite_the_regulation() {
    for r in [
        &Art8Phase1Ruleset as &dyn Ruleset,
        &Art8Phase2Ruleset as &dyn Ruleset,
    ] {
        let basis = r.regulatory_basis();
        assert_eq!(basis.regulation, "EU 2023/1542");
        assert!(basis.article.contains("Art. 8"), "{}", basis.article);
        assert!(basis.source_url.is_some());
    }
    assert_eq!(
        Art8Phase1Ruleset.regulatory_basis().superseded_by,
        Some("battery-recycled-content-art8-3"),
        "a closed phase must name its successor or the audit chain dead-ends"
    );
}
