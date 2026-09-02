//! Cross-ruleset metadata and effective-date guards for the stub rulesets, plus
//! the bundle-fill path through [`FilledRepairabilityRuleset`].

use chrono::{DateTime, TimeZone, Utc};
use dpp_rules::bundle::{
    AcceptancePolicy, JwsVerify, RulesetAcceptance, RulesetError, RulesetManifest, SignedBundle,
    content_hash, verify_bundle,
};

use super::*;
use crate::clock::AssessmentClock;
use crate::error::CalcError;
use crate::parameters::RulesetParameters;
use crate::repairability::calculate;
use crate::repairability::parameters::RepairabilityInputs;
use crate::ruleset::{Effectivity, ParameterBasis, Ruleset};

fn valid_inputs() -> RepairabilityInputs {
    RepairabilityInputs {
        disassembly: 2,
        spare_parts: 2,
        repair_info: 2,
        diagnostic_tools: 2,
        software_updatability: 2,
        customer_support: 2,
    }
}

#[test]
fn stub_rulesets_expose_consistent_metadata() {
    let rulesets: [&dyn RepairabilityRuleset; 3] =
        [&LaptopRuleset, &DisplaysRuleset, &WashingMachineRuleset];
    for rs in rulesets {
        let w = rs.weights();
        let sum = w.disassembly
            + w.spare_parts
            + w.repair_info
            + w.diagnostic_tools
            + w.software_updatability
            + w.customer_support;
        assert!((sum - 1.0).abs() < 1e-9, "weights must sum to 1.0");
        assert_eq!(rs.thresholds().a, 8.5);
        assert!(!rs.id().0.is_empty());
        assert!(!rs.version().0.is_empty());
        assert!(!rs.regulatory_basis().regulation.is_empty());
        // These acts are Pending, so they govern no date at all.
        assert!(matches!(rs.effectivity(), Effectivity::Pending { .. }));
    }
}

#[test]
fn calculating_with_a_pending_ruleset_is_rejected_as_undetermined() {
    // Laptop/Displays/Washing all await an unadopted ESPR delegated act, so
    // calculate() must refuse them — and as *undetermined*, not "not yet
    // effective". There is no date to be waiting for.
    let clock =
        AssessmentClock::placed_on(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid"));
    for result in [
        calculate(&valid_inputs(), &LaptopRuleset, clock),
        calculate(&valid_inputs(), &DisplaysRuleset, clock),
        calculate(&valid_inputs(), &WashingMachineRuleset, clock),
    ] {
        assert!(matches!(result, Err(CalcError::RulesetUndetermined { .. })));
    }
}

/// The signature check is exercised by `dpp-rules`' own tests against both
/// outcomes. What is under test here is what happens *after* a bundle
/// verifies, so this stands in for the engine-side EdDSA adapter.
struct SignatureAccepted;
impl JwsVerify for SignatureAccepted {
    fn verify_eddsa(&self, _jws: &str, _key: &str) -> Result<bool, RulesetError> {
        Ok(true)
    }
}

fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, 0, 0, 0)
        .single()
        .expect("real instant")
}

/// A JWS-shaped string whose payload segment decodes to `m`. Header and
/// signature are placeholders: `verify_bundle` delegates signature checking
/// to the injected verifier and only decodes the payload itself.
fn jws_carrying(m: &RulesetManifest) -> String {
    use base64::Engine;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(m).expect("manifest serialises"));
    format!("aGVhZGVy.{payload}.c2ln")
}

/// A bundle offering `content`, signed-shaped and internally consistent.
fn bundle_offering(content: serde_json::Value) -> SignedBundle {
    let manifest = RulesetManifest {
        bundle_version: "2026-Q3.1".to_owned(),
        effective_date: at(2026, 7, 1),
        act_citations: vec![],
        schema_versions: Default::default(),
        content_sha256: content_hash(&content).expect("test content hashes"),
    };
    SignedBundle {
        manifest_jws: jws_carrying(&manifest),
        content,
    }
}

fn accept(bundle: &SignedBundle) -> RulesetAcceptance {
    verify_bundle(
        bundle,
        "pinned-publisher-key",
        &SignatureAccepted,
        &AcceptancePolicy {
            now: at(2026, 8, 1),
            in_force: None,
        },
    )
    .expect("a well-formed, effective bundle verifies")
}

/// A bundle slice for the heuristic, with weights that sum to 1.0 but
/// deliberately differ from the compiled-in ones.
fn heuristic_slice(weights: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "rulesets": { "repairability-heuristic-v1": { "weights": weights } }
    })
}

fn even_weights() -> serde_json::Value {
    serde_json::json!({
        "disassembly": 0.5,
        "spareParts": 0.1,
        "repairInfo": 0.1,
        "diagnosticTools": 0.1,
        "softwareUpdatability": 0.1,
        "customerSupport": 0.1,
    })
}

fn in_force() -> AssessmentClock {
    AssessmentClock::placed_on(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
}

/// Inputs chosen so re-weighting moves the score: disassembly is scored 2
/// and everything else 0, so the result is exactly the disassembly weight's
/// share of the scale.
fn disassembly_only() -> RepairabilityInputs {
    RepairabilityInputs {
        disassembly: 2,
        spare_parts: 0,
        repair_info: 0,
        diagnostic_tools: 0,
        software_updatability: 0,
        customer_support: 0,
    }
}

/// The end-to-end path this crate was missing: a signed bundle verifies, its
/// parameters fill a ruleset, and a calculation run against that ruleset
/// produces a different answer and says so on the receipt.
#[test]
fn a_verified_bundle_changes_the_answer_and_the_receipt_says_so() {
    let baseline = calculate(
        &disassembly_only(),
        &SimplifiedRepairabilityHeuristic,
        in_force(),
    )
    .expect("baseline calculation");

    let bundle = bundle_offering(heuristic_slice(even_weights()));
    let acceptance = accept(&bundle);
    let filled = FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
        .expect("the bundle offers this ruleset")
        .expect("and adopting it succeeds");

    let after = calculate(&disassembly_only(), &filled, in_force()).expect("filled calculation");

    // 2 × 0.25 × 5 = 2.5 baseline; 2 × 0.50 × 5 = 5.0 filled.
    assert!(
        (baseline.numeric_score - 2.5).abs() < 1e-9,
        "baseline score: {}",
        baseline.numeric_score
    );
    assert!(
        (after.numeric_score - 5.0).abs() < 1e-9,
        "filled score: {}",
        after.numeric_score
    );

    let signed_over = content_hash(&bundle.content).expect("content hashes");
    let r = &after.receipt;
    assert_eq!(
        r.ruleset_id, "repairability-heuristic-v1",
        "identity is unchanged"
    );
    assert_eq!(r.bundle_version.as_deref(), Some("2026-Q3.1"));
    assert_eq!(
        r.bundle_content_sha256.as_deref(),
        Some(signed_over.as_str()),
        "the bundle hash on the receipt must be the one the manifest committed to"
    );
    assert_eq!(
        r.ruleset_content_sha256,
        filled
            .parameters()
            .content_sha256()
            .expect("filled parameters hash"),
        "the parameter hash must describe the numbers actually used"
    );
    assert_ne!(
        r.ruleset_content_sha256, baseline.receipt.ruleset_content_sha256,
        "a receipt computed from filled parameters must not hash to the baseline's"
    );
}

/// A baseline receipt still pins its numbers, and names no bundle.
///
/// This is the case that had no evidence at all before: `bundle_version` is
/// `None` for every compiled-in ruleset, so without the parameter hash a
/// baseline determination recorded nothing about the figures behind it.
#[test]
fn a_baseline_receipt_pins_its_numbers_without_naming_a_bundle() {
    let result = calculate(
        &disassembly_only(),
        &SimplifiedRepairabilityHeuristic,
        in_force(),
    )
    .expect("baseline calculation");

    assert_eq!(result.receipt.bundle_version, None);
    assert_eq!(result.receipt.bundle_content_sha256, None);
    assert_eq!(
        result.receipt.ruleset_content_sha256,
        SimplifiedRepairabilityHeuristic
            .parameters()
            .content_sha256()
            .expect("baseline parameters hash"),
    );
}

/// Compiled-in defaults are a legitimate way to start and not a source to
/// take numbers from.
///
/// `unverified_baseline` is public because a node with no configured channel
/// needs it; that same constructor would otherwise be a way to hand the fill
/// path bytes that nothing authenticated.
#[test]
fn an_unverified_acceptance_may_not_supply_parameters() {
    let content = heuristic_slice(even_weights());
    let manifest = RulesetManifest {
        bundle_version: "local".to_owned(),
        effective_date: at(2026, 1, 1),
        act_citations: vec![],
        schema_versions: Default::default(),
        content_sha256: content_hash(&content).expect("test content hashes"),
    };
    let acceptance = RulesetAcceptance::unverified_baseline(manifest, content);

    let err = FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
        .expect_err("an unverified acceptance must be refused");

    assert!(
        matches!(err, CalcError::UnverifiedBundle { ref ruleset_id }
            if ruleset_id == "repairability-heuristic-v1"),
        "{err}"
    );
}

/// A `rulesets` key that is not an object is named, not read as silence.
///
/// `Value::get` answers `None` to a string, an array and an absent key alike, so
/// without this the bundle would appear to apply cleanly while every slice in it
/// was skipped — the same silent-success failure the unknown-group check exists
/// to prevent, one level up.
#[test]
fn a_malformed_rulesets_key_is_not_read_as_silence() {
    let acceptance = accept(&bundle_offering(serde_json::json!({
        "rulesets": ["repairability-heuristic-v1"]
    })));

    let Err(err) =
        FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
    else {
        panic!("a non-object 'rulesets' must be refused, not treated as an absent key");
    };
    assert!(
        err.to_string().contains("array") && err.to_string().contains("2026-Q3.1"),
        "must say what it found and in which bundle: {err}"
    );
}

/// A bundle that says nothing about a ruleset leaves it alone. Not an error.
#[test]
fn a_bundle_silent_on_this_ruleset_leaves_it_alone() {
    let bundle = bundle_offering(serde_json::json!({
        "rulesets": { "some-other-ruleset": { "weights": {} } }
    }));
    let acceptance = accept(&bundle);

    let adopted = FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
        .expect("silence is not an error");
    assert!(adopted.is_none());
}

/// Weights that do not sum to 1.0 would put the score off the 0–10 scale the
/// band thresholds are written against — an A for a product that earned a C.
#[test]
fn weights_that_do_not_sum_to_one_are_refused() {
    let bundle = bundle_offering(heuristic_slice(serde_json::json!({
        "disassembly": 0.5,
        "spareParts": 0.5,
        "repairInfo": 0.5,
        "diagnosticTools": 0.5,
        "softwareUpdatability": 0.5,
        "customerSupport": 0.5,
    })));
    let acceptance = accept(&bundle);

    let err = FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
        .expect_err("weights summing to 3.0 must be refused");
    assert!(err.to_string().contains("sum to 3"), "{err}");
}

/// Bands that are not strictly descending let a score qualify for two of
/// them, and the first branch taken silently wins.
#[test]
fn band_thresholds_that_are_not_descending_are_refused() {
    let bundle = bundle_offering(serde_json::json!({
        "rulesets": {
            "repairability-heuristic-v1": {
                "thresholds": { "a": 5.0, "b": 7.0, "c": 5.5, "d": 4.0 }
            }
        }
    }));
    let acceptance = accept(&bundle);

    let err = FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
        .expect_err("non-descending bands must be refused");
    assert!(err.to_string().contains("strictly descending"), "{err}");
}

/// An unrecognised member inside a group is refused, not dropped.
///
/// The group name is right, so the fill-level unknown-group check passes it
/// through; what catches this is `deny_unknown_fields` on the weights
/// struct. Both shapes are asserted because only one of them is otherwise
/// silent:
///
/// - a **misspelling** leaves a required field missing, so serde refuses it
///   either way — but without `deny_unknown_fields` the message reads
///   `missing field spareParts`, naming what is absent rather than the
///   `sparParts` the bundle author actually wrote;
/// - an **extra** key alongside all six real ones is dropped in silence, the
///   fill succeeds, and nothing records that a line of the bundle did
///   nothing. That case has no other backstop.
#[test]
fn an_unrecognised_weight_is_refused_not_dropped() {
    let misspelled = serde_json::json!({
        "disassembly": 0.25,
        "sparParts": 0.15,
        "repairInfo": 0.15,
        "diagnosticTools": 0.15,
        "softwareUpdatability": 0.15,
        "customerSupport": 0.15,
    });
    let extra = serde_json::json!({
        "disassembly": 0.25,
        "spareParts": 0.15,
        "repairInfo": 0.15,
        "diagnosticTools": 0.15,
        "softwareUpdatability": 0.15,
        "customerSupport": 0.15,
        "batteryReplaceability": 0.0,
    });

    for (label, weights, key) in [
        ("misspelled", misspelled, "sparParts"),
        ("extra", extra, "batteryReplaceability"),
    ] {
        let acceptance = accept(&bundle_offering(heuristic_slice(weights)));
        let Err(err) =
            FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
        else {
            panic!("a {label} weight must be refused, not accepted");
        };
        assert!(
            err.to_string().contains(key),
            "the {label} case must name '{key}': {err}"
        );
    }
}

/// Filling does not launder a heuristic into law.
///
/// The numbers arrived through a signed channel, which says who sent them
/// and not what they are. If a fill could raise the basis to `Sourced`, a
/// later bundle would be refused on the strength of an earlier one's say-so,
/// and this ruleset's output — which its own basis calls non-regulatory —
/// would start reading as an act's.
#[test]
fn a_filled_ruleset_does_not_become_law() {
    let bundle = bundle_offering(heuristic_slice(even_weights()));
    let acceptance = accept(&bundle);
    let filled = FilledRepairabilityRuleset::adopt(&SimplifiedRepairabilityHeuristic, &acceptance)
        .expect("offered")
        .expect("adopted");

    assert_eq!(filled.parameter_basis(), ParameterBasis::Assumed);
    assert!(
        filled
            .regulatory_basis()
            .regulation
            .contains("Non-regulatory"),
        "a filled heuristic must still disclaim regulatory conformance"
    );
    assert!(
        crate::parameters::fill(&filled, &RulesetParameters::new()).is_ok(),
        "still fillable — a bundle must not be able to lock the next one out"
    );
}
