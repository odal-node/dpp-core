//! Golden-vector regression tests for the simplified repairability heuristic.
//!
//! Each vector is a hand-verified (disassembly, spare_parts, repair_info,
//! diagnostic_tools, software_updatability, customer_support) tuple with the
//! expected A–E band and approximate numeric score. These vectors pin the
//! heuristic's arithmetic only — they are **not** EU 2023/1669 conformance
//! vectors (the heuristic is non-regulatory; see `mod.rs`).
//!
//! Heuristic weights: disassembly=0.25, all other five parameters=0.15 each.
//! Scale factor: ×5 to reach 0–10.

use super::{
    DisplaysRuleset, RepairabilityClass, SimplifiedRepairabilityHeuristic, WashingMachineRuleset,
    calculate, parameters::RepairabilityInputs, thresholds::LaptopRuleset,
};
use crate::error::CalcError;
use crate::ruleset::Ruleset;

fn run(d: u8, sp: u8, ri: u8, dt: u8, su: u8, cs: u8) -> super::RepairabilityResult {
    let inputs = RepairabilityInputs {
        disassembly: d,
        spare_parts: sp,
        repair_info: ri,
        diagnostic_tools: dt,
        software_updatability: su,
        customer_support: cs,
    };
    calculate(&inputs, &SimplifiedRepairabilityHeuristic).expect("valid inputs")
}

const EPS: f64 = 1e-9;

#[test]
fn all_2_is_grade_a_with_score_10() {
    // (2×0.25 + 2×0.15 + 2×0.15 + 2×0.15 + 2×0.15 + 2×0.15)×5 = 2.0×1.0×5 = 10.0
    let r = run(2, 2, 2, 2, 2, 2);
    assert_eq!(r.class, RepairabilityClass::A);
    assert!(
        (r.numeric_score - 10.0).abs() < EPS,
        "score={}",
        r.numeric_score
    );
}

#[test]
fn all_0_is_grade_e_with_score_0() {
    let r = run(0, 0, 0, 0, 0, 0);
    assert_eq!(r.class, RepairabilityClass::E);
    assert!(
        (r.numeric_score - 0.0).abs() < EPS,
        "score={}",
        r.numeric_score
    );
}

#[test]
fn high_primary_criteria_is_grade_b() {
    // (2×0.25 + 2×0.15 + 2×0.15 + 1×0.15 + 1×0.15 + 1×0.15)×5
    //   = (0.50+0.30+0.30+0.15+0.15+0.15)×5 = 1.55×5 = 7.75 → B (7.0 ≤ 7.75 < 8.5)
    let r = run(2, 2, 2, 1, 1, 1);
    assert_eq!(r.class, RepairabilityClass::B);
    assert!(
        (r.numeric_score - 7.75).abs() < EPS,
        "score={}",
        r.numeric_score
    );
}

#[test]
fn moderate_profile_is_grade_c_at_threshold() {
    // (2×0.25 + 1×0.15 + 1×0.15 + 1×0.15 + 1×0.15 + 0×0.15)×5
    //   = (0.50+0.15+0.15+0.15+0.15)×5 = 1.10×5 = 5.5 → C (at threshold)
    let r = run(2, 1, 1, 1, 1, 0);
    assert_eq!(r.class, RepairabilityClass::C);
    assert!(
        (r.numeric_score - 5.5).abs() < EPS,
        "score={}",
        r.numeric_score
    );
}

#[test]
fn weak_profile_is_grade_d() {
    // (2×0.25 + 1×0.15 + 1×0.15 + 0×0.15 + 0×0.15 + 1×0.15)×5
    //   = (0.50+0.15+0.15+0+0+0.15)×5 = 0.95×5 = 4.75 → D
    let r = run(2, 1, 1, 0, 0, 1);
    assert_eq!(r.class, RepairabilityClass::D);
    assert!(
        (r.numeric_score - 4.75).abs() < EPS,
        "score={}",
        r.numeric_score
    );
}

#[test]
fn low_spare_parts_only_is_grade_e() {
    // (1×0.25 + 1×0.15 + 1×0.15 + 0 + 0 + 0)×5 = 0.55×5 = 2.75 → E (< 4.0)
    let r = run(1, 1, 1, 0, 0, 0);
    assert_eq!(r.class, RepairabilityClass::E);
    assert!(
        (r.numeric_score - 2.75).abs() < EPS,
        "score={}",
        r.numeric_score
    );
}

#[test]
fn out_of_range_parameter_is_rejected() {
    let inputs = RepairabilityInputs {
        disassembly: 3, // invalid
        spare_parts: 1,
        repair_info: 1,
        diagnostic_tools: 0,
        software_updatability: 0,
        customer_support: 0,
    };
    assert!(calculate(&inputs, &SimplifiedRepairabilityHeuristic).is_err());
}

#[test]
fn contributions_sum_to_numeric_score() {
    let r = run(2, 2, 1, 1, 2, 1);
    let c = &r.contributions;
    let sum = c.disassembly
        + c.spare_parts
        + c.repair_info
        + c.diagnostic_tools
        + c.software_updatability
        + c.customer_support;
    assert!(
        (sum - r.numeric_score).abs() < EPS,
        "contributions sum {sum} ≠ numeric_score {}",
        r.numeric_score
    );
}

#[test]
fn contributions_are_individually_correct() {
    // All params = 2 → each contribution = 2 × weight × 5
    let r = run(2, 2, 2, 2, 2, 2);
    let c = &r.contributions;
    assert!((c.disassembly - 2.0 * 0.25 * 5.0).abs() < EPS);
    assert!((c.spare_parts - 2.0 * 0.15 * 5.0).abs() < EPS);
    assert!((c.repair_info - 2.0 * 0.15 * 5.0).abs() < EPS);
    assert!((c.diagnostic_tools - 2.0 * 0.15 * 5.0).abs() < EPS);
    assert!((c.software_updatability - 2.0 * 0.15 * 5.0).abs() < EPS);
    assert!((c.customer_support - 2.0 * 0.15 * 5.0).abs() < EPS);
}

// ── Cross-field validation ────────────────────────────────────────────────

#[test]
fn disassembly_zero_spare_parts_nonzero_is_cross_field_violation() {
    let inputs = RepairabilityInputs {
        disassembly: 0,
        spare_parts: 1, // incoherent: can't use parts if device can't be opened
        repair_info: 1,
        diagnostic_tools: 0,
        software_updatability: 0,
        customer_support: 0,
    };
    let err =
        calculate(&inputs, &SimplifiedRepairabilityHeuristic).expect_err("should fail cross-check");
    assert!(
        matches!(err, CalcError::CrossFieldViolation(_)),
        "expected CrossFieldViolation, got {err:?}"
    );
}

#[test]
fn disassembly_zero_spare_parts_zero_is_valid() {
    let inputs = RepairabilityInputs {
        disassembly: 0,
        spare_parts: 0, // coherent: no instructions → no parts
        repair_info: 0,
        diagnostic_tools: 0,
        software_updatability: 0,
        customer_support: 0,
    };
    assert!(calculate(&inputs, &SimplifiedRepairabilityHeuristic).is_ok());
}

#[test]
fn disassembly_nonzero_allows_all_spare_parts_scores() {
    for sp in [0u8, 1, 2] {
        let inputs = RepairabilityInputs {
            disassembly: 1,
            spare_parts: sp,
            repair_info: 1,
            diagnostic_tools: 0,
            software_updatability: 0,
            customer_support: 0,
        };
        assert!(
            calculate(&inputs, &SimplifiedRepairabilityHeuristic).is_ok(),
            "disassembly=1, spare_parts={sp} should be valid"
        );
    }
}

// ── Stub rulesets — registry/basis invariants ─────────────────────────────

#[test]
fn all_concrete_rulesets_have_non_empty_regulatory_basis() {
    // Every concrete ruleset must have a non-empty regulation + article citation
    // so that a notified body can locate the authoritative source.
    for (name, r) in [
        (
            "SimplifiedRepairabilityHeuristic",
            &SimplifiedRepairabilityHeuristic as &dyn Ruleset,
        ),
        ("LaptopRuleset", &LaptopRuleset),
        ("DisplaysRuleset", &DisplaysRuleset as &dyn Ruleset),
        (
            "WashingMachineRuleset",
            &WashingMachineRuleset as &dyn Ruleset,
        ),
    ] {
        let b = r.regulatory_basis();
        assert!(
            !b.regulation.is_empty(),
            "{name}: regulatory_basis.regulation is empty"
        );
        assert!(
            !b.article.is_empty(),
            "{name}: regulatory_basis.article is empty"
        );
    }
}

#[test]
fn receipt_contains_correct_ruleset_id_and_output_hash() {
    let inputs = RepairabilityInputs {
        disassembly: 1,
        spare_parts: 1,
        repair_info: 1,
        diagnostic_tools: 1,
        software_updatability: 1,
        customer_support: 1,
    };
    let r = calculate(&inputs, &SimplifiedRepairabilityHeuristic).unwrap();
    assert_eq!(r.receipt.ruleset_id, "repairability-heuristic-v1");
    assert!(!r.receipt.input_hash.is_empty());
    assert!(!r.receipt.output_hash.is_empty());
    assert!(!r.receipt.receipt_id.is_nil());
}
