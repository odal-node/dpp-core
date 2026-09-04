//! Unit tests for the methodology-agnostic spine.

use super::ruleset::{Effectivity, Ruleset, RulesetId};
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
///
/// `SaysNothing` implements the five required methods and nothing else.
/// `parameters()` is among them deliberately — it has no default, because a
/// ruleset that declared no numbers would hash to the same empty set as every
/// other silent one and `ruleset_content_sha256` would attest to nothing.
#[test]
fn an_unclassified_ruleset_defaults_to_carrying_law() {
    use super::parameters::RulesetParameters;
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
        fn parameters(&self) -> RulesetParameters {
            RulesetParameters::new()
        }
    }

    assert_eq!(SaysNothing.parameter_basis(), ParameterBasis::Sourced);
    assert!(SaysNothing.bundle_provenance().is_none());
}

// ── Fill, never override ─────────────────────────────────────────────────────

/// Replacement weights that are valid on their own terms — they sum to 1.0 —
/// so nothing but provenance can be the reason a fill is refused.
fn offered_weights() -> crate::parameters::RulesetParameters {
    crate::parameters::RulesetParameters::new()
        .with(
            "weights",
            &serde_json::json!({
                "disassembly": 0.30,
                "spareParts": 0.14,
                "repairInfo": 0.14,
                "diagnosticTools": 0.14,
                "softwareUpdatability": 0.14,
                "customerSupport": 0.14,
            }),
        )
        .expect("test weights serialise")
}

/// A bundle may never rewrite a threshold that comes from an act.
///
/// The Art. 8 shares are the case the whole distinction exists to protect: they
/// were read out of Regulation (EU) 2023/1542 and verified against the Official
/// Journal, and a channel that could replace them could make a node report a
/// legal minimum that is not the law.
#[test]
fn a_bundle_may_not_override_a_threshold_that_comes_from_law() {
    use crate::recycled_content::Art8Phase1Ruleset;

    let err = crate::parameters::fill(&Art8Phase1Ruleset, &offered_weights())
        .expect_err("a Sourced ruleset must refuse to be filled");

    assert!(
        matches!(err, CalcError::SourcedParametersNotFillable { ref ruleset_id }
            if ruleset_id == "battery-recycled-content-art8-2"),
        "must name the ruleset it protected: {err}"
    );
}

/// The other half of the same rule: a placeholder *is* replaceable.
///
/// Without this the refusal above would be indistinguishable from a channel
/// that refuses everything, which is what a presence-based test would have
/// produced.
#[test]
fn a_bundle_may_fill_numbers_that_are_ours() {
    use crate::repairability::thresholds::SimplifiedRepairabilityHeuristic;

    let baseline = SimplifiedRepairabilityHeuristic.parameters();
    let filled = crate::parameters::fill(&SimplifiedRepairabilityHeuristic, &offered_weights())
        .expect("an Assumed ruleset accepts a fill");

    assert_ne!(
        filled.content_sha256().expect("filled parameters hash"),
        baseline.content_sha256().expect("baseline parameters hash"),
        "a fill that changed a weight must change the parameter hash"
    );
    assert_eq!(
        filled.group("thresholds"),
        baseline.group("thresholds"),
        "a group the bundle did not offer must be left alone"
    );
}

/// An unrecognised group name is named, not dropped.
///
/// Serde's default would silently discard it: the bundle reports success, the
/// numbers do not move, and every receipt agrees with the numbers — so the
/// operator has no surface on which to notice.
#[test]
fn an_unknown_parameter_group_is_refused_by_name() {
    use crate::repairability::thresholds::SimplifiedRepairabilityHeuristic;

    let typo = crate::parameters::RulesetParameters::new()
        .with("weight", &serde_json::json!({}))
        .expect("test group serialises");

    let err = crate::parameters::fill(&SimplifiedRepairabilityHeuristic, &typo)
        .expect_err("an undeclared group must be refused");

    let msg = err.to_string();
    assert!(
        msg.contains("'weight'") && msg.contains("weights"),
        "must name the offered key and list the real ones: {msg}"
    );
}

/// A group whose JSON type changed is refused at the fill, where the bundle is
/// still in the message.
#[test]
fn a_group_offered_as_the_wrong_type_is_refused() {
    use crate::repairability::thresholds::SimplifiedRepairabilityHeuristic;

    let wrong = crate::parameters::RulesetParameters::new()
        .with("weights", &"0.25,0.15,0.15,0.15,0.15,0.15")
        .expect("test group serialises");

    let err = crate::parameters::fill(&SimplifiedRepairabilityHeuristic, &wrong)
        .expect_err("a type change must be refused");

    assert!(
        matches!(err, CalcError::ParameterGroupTypeMismatch { expected, got, .. }
            if expected == "object" && got == "string"),
        "must say which types disagreed: {err}"
    );
}

/// Every ruleset declares the numbers it computes with.
///
/// `parameters()` has no default precisely so this cannot drift: a ruleset that
/// declared nothing would still produce a `ruleset_content_sha256`, but it would
/// be the hash of the empty set — identical for every silent ruleset, and
/// therefore evidence of nothing. This is also the coverage check the
/// `Sourced`-default tripwire needs, since both iterate `all_rulesets()`.
#[test]
fn every_ruleset_declares_the_numbers_it_computes_with() {
    let mut hashes: std::collections::HashMap<String, &str> = std::collections::HashMap::new();

    for ruleset in crate::ruleset_registry::all_rulesets() {
        let parameters = ruleset.parameters();
        assert!(
            !parameters.is_empty(),
            "ruleset '{}' declares no parameters — its receipts would hash the empty set",
            ruleset.id().0
        );

        let hash = parameters
            .content_sha256()
            .unwrap_or_else(|e| panic!("ruleset '{}' parameters must hash: {e}", ruleset.id().0));

        if let Some(other) = hashes.insert(hash, ruleset.id().0) {
            panic!(
                "rulesets '{}' and '{}' hash to the same parameters — one of them is \
                 declaring the other's numbers",
                other,
                ruleset.id().0
            );
        }
    }
}
