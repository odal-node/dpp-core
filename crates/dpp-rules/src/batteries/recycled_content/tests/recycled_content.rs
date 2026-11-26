//! Tests of the `recycled_content` module.

// BOUNDARY-EXCEPTION(taxonomy): "Phase 1" and "Phase 2" are the stages of
// Regulation (EU) 2023/1542 Art. 8(2) and 8(3), not a roadmap.

use super::super::*;
use crate::common::date::CalendarDate;

fn all_metals(co: f64, pb: f64, li: f64, ni: f64) -> RecycledContentInput {
    RecycledContentInput {
        cobalt_pct: Some(co),
        lead_pct: Some(pb),
        lithium_pct: Some(li),
        nickel_pct: Some(ni),
    }
}

// ── Art. 8 phase selection — golden vectors ──────────────────────────────

#[test]
fn battery_placed_before_phase1_stays_unbound_however_late_it_is_assessed() {
    // A battery lawfully placed on the EU market on 1 Jun 2030, assessed in
    // 2033. Art. 8(2) attaches at placing on the market, so the 2031 minimums
    // never reach it. Deriving the phase from the assessment date instead
    // would report this battery as non-compliant from 18 Aug 2031 onwards.
    let placed = CalendarDate::new(2030, 6, 1);
    assert_eq!(
        art8_phase_for(Art8Category::IndustrialEvSli, placed, Art8SecondLife::None),
        Art8Phase::NotYetBinding
    );
}

#[test]
fn phase1_boundary_is_inclusive_of_18_august_2031() {
    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2031, 8, 17),
            Art8SecondLife::None
        ),
        Art8Phase::NotYetBinding
    );
    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2031, 8, 18),
            Art8SecondLife::None
        ),
        Art8Phase::Phase1
    );
}

#[test]
fn phase2_boundary_is_inclusive_of_18_august_2036() {
    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2036, 8, 17),
            Art8SecondLife::None
        ),
        Art8Phase::Phase1
    );
    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2036, 8, 18),
            Art8SecondLife::None
        ),
        Art8Phase::Phase2
    );
}

#[test]
fn lmt_is_outside_phase1_entirely() {
    // Art. 8(2) does not name LMT batteries; Art. 8(3) does. An LMT battery
    // placed on the market the very day Phase 1 begins is still unbound.
    assert_eq!(
        art8_phase_for(
            Art8Category::Lmt,
            CalendarDate::new(2031, 8, 18),
            Art8SecondLife::None
        ),
        Art8Phase::NotYetBinding
    );
    assert_eq!(
        art8_phase_for(
            Art8Category::Lmt,
            CalendarDate::new(2036, 8, 18),
            Art8SecondLife::None
        ),
        Art8Phase::Phase2
    );
}

#[test]
fn out_of_scope_categories_are_never_bound() {
    // Portable batteries, at any date, including well past Phase 2.
    assert_eq!(
        art8_phase_for(
            Art8Category::NotCovered,
            CalendarDate::new(2040, 1, 1),
            Art8SecondLife::None
        ),
        Art8Phase::NotCovered
    );
}

#[test]
fn not_covered_and_not_yet_binding_are_distinguishable() {
    // The whole point of a four-outcome enum: an operator told "not covered"
    // and one told "not yet" have different obligations, and neither has a
    // shortfall.
    let portable = art8_phase_for(
        Art8Category::NotCovered,
        CalendarDate::new(2033, 1, 1),
        Art8SecondLife::None,
    );
    let early_ev = art8_phase_for(
        Art8Category::IndustrialEvSli,
        CalendarDate::new(2030, 1, 1),
        Art8SecondLife::None,
    );
    assert_ne!(portable, early_ev);
}

#[test]
fn selected_phase_agrees_with_the_target_constants() {
    // 16 % cobalt clears Phase 1 and fails Phase 2. The phase selector and
    // the constants it selects between must not disagree about which set
    // is in force on a given placing-on-market date.
    let input = all_metals(16.0, 85.0, 6.0, 6.0);

    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2032, 1, 1),
            Art8SecondLife::None
        ),
        Art8Phase::Phase1
    );
    assert!(art8_shortfalls_2031(&input).is_empty());

    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2037, 1, 1),
            Art8SecondLife::None
        ),
        Art8Phase::Phase2
    );
    assert!(
        art8_shortfalls_2036(&input)
            .iter()
            .any(|s| s.material == "cobalt")
    );
}

// ── Category mapping ─────────────────────────────────────────────────────

#[test]
fn portable_is_not_covered_and_lmt_is_its_own_category() {
    assert_eq!(
        art8_category_for("portable", None),
        Art8Category::NotCovered
    );
    assert_eq!(
        art8_category_for("PORTABLE", None),
        Art8Category::NotCovered
    );
    assert_eq!(art8_category_for("lmt", None), Art8Category::Lmt);
    assert_eq!(art8_category_for(" LMT ", None), Art8Category::Lmt);
}

#[test]
fn industrial_is_covered_only_above_two_kwh() {
    assert_eq!(
        art8_category_for("industrial", Some(2.5)),
        Art8Category::IndustrialEvSli
    );
    // Art. 8 says "greater than 2 kWh" — exactly 2 kWh is out.
    assert_eq!(
        art8_category_for("industrial", Some(2.0)),
        Art8Category::NotCovered
    );
    assert_eq!(
        art8_category_for("industrial", None),
        Art8Category::NotCovered
    );
    // A non-finite capacity cannot demonstrate the threshold is met.
    assert_eq!(
        art8_category_for("industrial", Some(f64::NAN)),
        Art8Category::NotCovered
    );
}

#[test]
fn ev_sli_and_unknown_types_are_assessed() {
    for t in ["ev", "sli", "starting-lighting-ignition", "", "mystery"] {
        assert_eq!(
            art8_category_for(t, None),
            Art8Category::IndustrialEvSli,
            "type {t:?} should be assessed"
        );
    }
}

// ── Art. 8(1) declaration duty ───────────────────────────────────────────

#[test]
fn declaration_is_certainly_not_due_before_its_floor() {
    // The one thing that *is* knowable while the delegated act is
    // unadopted: the real date is never earlier than the floor, so anything
    // placed on the market before it definitely owes nothing yet.
    assert_eq!(
        art8_declaration_duty_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2028, 8, 17)
        ),
        Art8DeclarationDuty::NotYetDue {
            not_before: CalendarDate::new(2028, 8, 18)
        }
    );
}

#[test]
fn declaration_is_undetermined_on_and_after_the_floor() {
    // Art. 8(1) applies from the floor "or 24 months after the date of entry
    // into force of the delegated act, whichever is the latest". The act is
    // unadopted, so on/after the floor the answer is genuinely unknown —
    // reporting it as "due" would assert a date the regulation does not set.
    let got =
        art8_declaration_duty_for(Art8Category::IndustrialEvSli, CalendarDate::new(2029, 1, 1));
    let Art8DeclarationDuty::Undetermined {
        not_before,
        empowerment,
    } = got
    else {
        panic!("expected Undetermined, got {got:?}");
    };
    assert_eq!(not_before, CalendarDate::new(2028, 8, 18));
    assert!(empowerment.contains("Art. 8(1)"));
}

#[test]
fn lmt_declaration_floor_is_five_years_later() {
    // Art. 8(1) second subparagraph: LMT from 18 Aug 2033, not 2028.
    assert_eq!(
        art8_declaration_duty_for(Art8Category::Lmt, CalendarDate::new(2030, 1, 1)),
        Art8DeclarationDuty::NotYetDue {
            not_before: CalendarDate::new(2033, 8, 18)
        }
    );
    assert!(matches!(
        art8_declaration_duty_for(Art8Category::Lmt, CalendarDate::new(2034, 1, 1)),
        Art8DeclarationDuty::Undetermined { .. }
    ));
}

#[test]
fn out_of_scope_categories_owe_no_declaration() {
    assert_eq!(
        art8_declaration_duty_for(Art8Category::NotCovered, CalendarDate::new(2040, 1, 1)),
        Art8DeclarationDuty::NotCovered
    );
}

#[test]
fn the_declaration_and_minimum_ladders_are_independent() {
    // A battery placed on the market in 2029 is past the declaration floor
    // but years short of the Art. 8(2) minimums. Conflating the two would
    // report a shortfall against a duty that does not yet exist.
    let placed = CalendarDate::new(2029, 1, 1);
    assert!(matches!(
        art8_declaration_duty_for(Art8Category::IndustrialEvSli, placed),
        Art8DeclarationDuty::Undetermined { .. }
    ));
    assert_eq!(
        art8_phase_for(Art8Category::IndustrialEvSli, placed, Art8SecondLife::None),
        Art8Phase::NotYetBinding
    );
}

// ── Target checks ────────────────────────────────────────────────────────

#[test]
fn exactly_at_2031_targets_passes() {
    let input = all_metals(16.0, 85.0, 6.0, 6.0);
    assert!(art8_shortfalls_2031(&input).is_empty());
}

#[test]
fn above_2031_targets_passes() {
    let input = all_metals(20.0, 90.0, 10.0, 10.0);
    assert!(art8_shortfalls_2031(&input).is_empty());
}

#[test]
fn below_2031_cobalt_flagged() {
    let input = all_metals(15.0, 85.0, 6.0, 6.0); // cobalt 15 < 16
    let shortfalls = art8_shortfalls_2031(&input);
    assert_eq!(shortfalls.len(), 1);
    assert_eq!(shortfalls[0].material, "cobalt");
    assert_eq!(shortfalls[0].required_pct, 16.0);
}

#[test]
fn multiple_shortfalls_all_returned() {
    let input = all_metals(10.0, 80.0, 3.0, 4.0); // all below
    assert_eq!(art8_shortfalls_2031(&input).len(), 4);
}

#[test]
fn undeclared_metals_not_flagged() {
    let input = RecycledContentInput {
        cobalt_pct: Some(20.0),
        lead_pct: None,
        lithium_pct: None,
        nickel_pct: None,
    };
    assert!(art8_shortfalls_2031(&input).is_empty());
}

#[test]
fn phase2_stricter_than_phase1() {
    // 16% cobalt passes 2031 but fails 2036 (target 26%)
    let input = all_metals(16.0, 85.0, 6.0, 6.0);
    assert!(art8_shortfalls_2031(&input).is_empty());
    let shortfalls = art8_shortfalls_2036(&input);
    assert!(shortfalls.iter().any(|s| s.material == "cobalt"));
}

#[test]
fn nan_cobalt_treated_as_shortfall() {
    let input = RecycledContentInput {
        cobalt_pct: Some(f64::NAN),
        lead_pct: None,
        lithium_pct: None,
        nickel_pct: None,
    };
    let shortfalls = art8_shortfalls_2031(&input);
    assert_eq!(shortfalls.len(), 1);
    assert_eq!(shortfalls[0].material, "cobalt");
}

#[test]
fn infinity_cobalt_treated_as_shortfall() {
    let input = RecycledContentInput {
        cobalt_pct: Some(f64::INFINITY),
        lead_pct: None,
        lithium_pct: None,
        nickel_pct: None,
    };
    let shortfalls = art8_shortfalls_2031(&input);
    assert_eq!(shortfalls.len(), 1);
    assert_eq!(shortfalls[0].material, "cobalt");
}

#[test]
fn lfp_regulates_lithium_only() {
    let m = chemistry_regulated_metals("LFP");
    assert!(m.lithium);
    assert!(!m.cobalt && !m.nickel && !m.lead);
    // case-insensitive
    assert_eq!(chemistry_regulated_metals("lfp"), m);
}

#[test]
fn nmc_and_nca_regulate_cobalt_lithium_nickel() {
    for chem in ["NMC", "NCA"] {
        let m = chemistry_regulated_metals(chem);
        assert!(m.cobalt && m.lithium && m.nickel);
        assert!(!m.lead);
    }
}

#[test]
fn lead_acid_regulates_lead_only() {
    let m = chemistry_regulated_metals("lead-acid");
    assert!(m.lead);
    assert!(!m.cobalt && !m.lithium && !m.nickel);
}

#[test]
fn unknown_chemistry_checks_all_metals() {
    let m = chemistry_regulated_metals("mystery-cell");
    assert!(m.cobalt && m.lithium && m.nickel && m.lead);
}

#[test]
fn positive_cobalt_on_lfp_is_a_conflict() {
    let c = recycled_content_chemistry_conflicts("LFP", Some(5.0), Some(12.0), None, None);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0], "cobalt");
}

#[test]
fn zero_cobalt_on_lfp_is_not_a_conflict() {
    // 0.0 declares "no recycled cobalt" — trivially true for an absent metal.
    let c = recycled_content_chemistry_conflicts("LFP", Some(0.0), Some(12.0), Some(0.0), None);
    assert!(c.is_empty(), "got: {c:?}");
}

#[test]
fn nmc_cobalt_and_nickel_declared_no_conflict() {
    let c = recycled_content_chemistry_conflicts("NMC", Some(16.0), Some(6.0), Some(8.0), None);
    assert!(c.is_empty(), "got: {c:?}");
}

#[test]
fn lead_declared_on_lfp_is_a_conflict() {
    let c = recycled_content_chemistry_conflicts("LFP", None, Some(12.0), None, Some(80.0));
    assert_eq!(c.len(), 1);
    assert_eq!(c[0], "lead");
}

#[test]
fn unknown_chemistry_never_conflicts() {
    let c =
        recycled_content_chemistry_conflicts("mystery", Some(5.0), Some(5.0), Some(5.0), Some(5.0));
    assert!(c.is_empty());
}

// ── Art. 8(4) — the second-life carve-out ────────────────────────────────
//
// Added 2026-08-27. Before it, a remanufactured battery was reported as
// bound by the minimum shares that Art. 8(4) removes.

/// Art. 8(4) disapplies paragraphs 1-3 outright, so it answers before
/// category or date. A battery deep inside Phase 2 is still exempt.
#[test]
fn a_second_life_battery_is_exempt_even_inside_phase_2() {
    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            CalendarDate::new(2040, 1, 1),
            Art8SecondLife::PlacedBeforeOperations,
        ),
        Art8Phase::ExemptSecondLife
    );
}

/// **Art. 8(4) is broader than Art. 10(4).** Its condition is prior
/// placement relative to the *operation*, not to the obligation dates, so a
/// battery placed in 2033 and remanufactured later is exempt from Art. 8 —
/// where the same battery is not exempt from Art. 10. Collapsing the two
/// carve-outs into one test would hide exactly this.
#[test]
fn art8_exemption_does_not_require_placement_before_the_phase_dates() {
    let placed_after_phase1_began = CalendarDate::new(2033, 5, 1);
    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            placed_after_phase1_began,
            Art8SecondLife::None,
        ),
        Art8Phase::Phase1,
        "an original battery placed in 2033 is bound by Art. 8(2)"
    );
    assert_eq!(
        art8_phase_for(
            Art8Category::IndustrialEvSli,
            placed_after_phase1_began,
            Art8SecondLife::PlacedBeforeOperations,
        ),
        Art8Phase::ExemptSecondLife,
        "the same battery, remanufactured, is exempt under Art. 8(4)"
    );
}

/// Exempt and NotCovered are different sentences and must stay distinct.
#[test]
fn exempt_is_not_the_same_answer_as_not_covered() {
    assert_ne!(Art8Phase::ExemptSecondLife, Art8Phase::NotCovered);
    assert_eq!(
        art8_phase_for(
            Art8Category::NotCovered,
            CalendarDate::new(2040, 1, 1),
            Art8SecondLife::PlacedBeforeOperations,
        ),
        Art8Phase::ExemptSecondLife,
        "Art. 8(4) is checked first, so it answers even for an out-of-scope category"
    );
}
