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

/// A ruleset that cites no source may not claim its numbers are law.
///
/// This is what makes the `Sourced` default safe. The default is deliberately
/// fail-closed — an unclassified ruleset reads as carrying law, so nothing
/// overwrites it — but that same default would let a *new* placeholder ruleset
/// silently claim legal provenance simply by not saying anything. Requiring a
/// citation for the `Sourced` claim closes that: forget to classify a stub, and
/// this fails.
///
/// Only the safety-relevant direction is asserted. A ruleset sourced from a
/// print-only standard with no URL is imaginable, and would want a deliberate
/// decision rather than a test that pre-emptively forbids it.
#[test]
fn an_unsourced_ruleset_may_not_claim_its_numbers_are_law() {
    use crate::kernel::ruleset::ParameterBasis;

    let mut assumed = 0_usize;
    for ruleset in crate::ruleset_registry::all_rulesets() {
        let basis = ruleset.parameter_basis();
        if basis == ParameterBasis::Assumed {
            assumed += 1;
            continue;
        }
        assert!(
            ruleset.regulatory_basis().source_url.is_some(),
            "ruleset '{}' claims Sourced parameters while citing no source — \
             either give its basis a source_url or classify it Assumed",
            ruleset.id().0
        );
    }

    assert!(
        assumed > 0,
        "no ruleset is Assumed, so this test proved nothing; the placeholder \
         rulesets that motivated the distinction have gone missing"
    );
}

/// The default is `Sourced`, and that is the fail-closed direction.
///
/// A ruleset that says nothing about its provenance must read as carrying law,
/// so anything gating on this refuses to touch it. The opposite default would
/// let an unclassified ruleset be overwritten silently, which is the one
/// outcome the distinction exists to prevent.
#[test]
fn an_unclassified_ruleset_defaults_to_carrying_law() {
    use super::ruleset::{ParameterBasis, RegulatoryBasis, Ruleset, RulesetId, RulesetVersion};

    struct SaysNothing;

    static ID: RulesetId = RulesetId("says-nothing");
    static VERSION: RulesetVersion = RulesetVersion("1.0.0");
    static EFFECTIVITY: Effectivity =
        Effectivity::pending("an instrument that has not been adopted", None);
    static BASIS: RegulatoryBasis = RegulatoryBasis {
        regulation: "unspecified",
        article: "unspecified",
        standard: None,
        technical_study: None,
        source_url: None,
        superseded_by: None,
    };

    impl Ruleset for SaysNothing {
        fn id(&self) -> &RulesetId {
            &ID
        }
        fn version(&self) -> &RulesetVersion {
            &VERSION
        }
        fn effectivity(&self) -> &Effectivity {
            &EFFECTIVITY
        }
        fn regulatory_basis(&self) -> &RegulatoryBasis {
            &BASIS
        }
    }

    assert_eq!(SaysNothing.parameter_basis(), ParameterBasis::Sourced);
}
