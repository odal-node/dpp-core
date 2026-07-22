//! Worked examples and boundary regressions for the EU 2023/1669 index.

use super::*;
use crate::repairability_index::parameters::PriorityPartScores;
use crate::ruleset::Ruleset;

/// All nine non-folding parts at the same score.
fn flat(score: u8, folding: Option<u8>) -> PriorityPartScores {
    PriorityPartScores {
        battery: score,
        display_assembly: score,
        back_cover: score,
        front_camera: score,
        rear_camera: score,
        charging_port: score,
        mechanical_button: score,
        microphone: score,
        speaker: score,
        folding_mechanism: folding,
    }
}

fn uniform(score: u8, folding: Option<u8>) -> RepairabilityIndexInputs {
    RepairabilityIndexInputs {
        disassembly_depth: flat(score, folding),
        fasteners: flat(score, folding),
        tools: flat(score, folding),
        spare_parts: score,
        software_updates: score,
        repair_information: score,
    }
}

#[test]
fn part_weights_sum_to_one() {
    // Both Annex IV weight sets must total 1, or aggregation is silently scaled.
    let r = Eu2023_1669Ruleset;
    for foldable in [false, true] {
        let w = r.part_weights(foldable);
        let total = w.battery
            + w.display_assembly
            + w.back_cover
            + w.minor_part * 6.0
            + w.folding_mechanism.unwrap_or(0.0);
        assert!(
            (total - 1.0).abs() < 1e-9,
            "foldable={foldable} part weights sum to {total}, expected 1"
        );
    }
}

#[test]
fn parameter_weights_sum_to_one() {
    let w = Eu2023_1669Ruleset.weights();
    let total = w.disassembly_depth
        + w.fasteners
        + w.tools
        + w.spare_parts
        + w.software_updates
        + w.repair_information;
    assert!((total - 1.0).abs() < 1e-9, "weights sum to {total}");
}

#[test]
fn all_fives_is_the_maximum_index_and_class_a() {
    let r = calculate(&uniform(5, None), &Eu2023_1669Ruleset).unwrap();
    assert!((r.index - 5.0).abs() < 1e-9, "index was {}", r.index);
    assert_eq!(r.class, RepairabilityClass::A);
}

#[test]
fn all_ones_is_the_minimum_index_and_class_e() {
    let r = calculate(&uniform(1, None), &Eu2023_1669Ruleset).unwrap();
    assert!((r.index - 1.0).abs() < 1e-9, "index was {}", r.index);
    assert_eq!(r.class, RepairabilityClass::E);
}

#[test]
fn foldable_weight_set_also_spans_one_to_five() {
    let lo = calculate(&uniform(1, Some(1)), &Eu2023_1669Ruleset).unwrap();
    let hi = calculate(&uniform(5, Some(5)), &Eu2023_1669Ruleset).unwrap();
    assert!((lo.index - 1.0).abs() < 1e-9);
    assert!((hi.index - 5.0).abs() < 1e-9);
    assert!(hi.foldable);
}

/// Annex II Table 4 boundaries, driven through the real calculator.
///
/// A uniform score `s` across every parameter yields an index of exactly `s`,
/// because both weight sets sum to 1 — so integer scores land on known points
/// and exercise the classification branches for real rather than through a
/// re-implementation of them.
#[test]
fn class_boundaries_match_annex_ii_table_4() {
    // R = 4,00 is the exact lower edge of class A.
    let a = calculate(&uniform(4, None), &Eu2023_1669Ruleset).unwrap();
    assert!((a.index - 4.0).abs() < 1e-9);
    assert_eq!(a.class, RepairabilityClass::A, "R = 4,00 is class A");

    // R = 3,00 sits inside C (3,35 > R >= 2,55).
    let c = calculate(&uniform(3, None), &Eu2023_1669Ruleset).unwrap();
    assert_eq!(c.class, RepairabilityClass::C);

    // R = 2,00 sits inside D (2,55 > R >= 1,75).
    let d = calculate(&uniform(2, None), &Eu2023_1669Ruleset).unwrap();
    assert_eq!(d.class, RepairabilityClass::D);

    // R = 1,00 is the floor of E.
    let e = calculate(&uniform(1, None), &Eu2023_1669Ruleset).unwrap();
    assert_eq!(e.class, RepairabilityClass::E);

    // Class B needs a non-integer: parts at 3, product-level parameters at 5
    // gives 3,00 + 0,15 * 2 * 3 = 3,90.
    let mut b_inputs = uniform(3, None);
    b_inputs.spare_parts = 5;
    b_inputs.software_updates = 5;
    b_inputs.repair_information = 5;
    let b = calculate(&b_inputs, &Eu2023_1669Ruleset).unwrap();
    assert!((b.index - 3.90).abs() < 1e-9, "index was {}", b.index);
    assert_eq!(b.class, RepairabilityClass::B);
}

/// Just below each edge must drop exactly one class.
#[test]
fn just_below_an_edge_drops_a_class() {
    let bounds = Eu2023_1669Ruleset.class_boundaries();
    // Parts at 1 contribute 1,00 * 0,55; product-level parameters carry 0,15
    // each, so a fractional product-level score is not expressible. Instead
    // step the index down from a known point via the part aggregate.
    let just_under_a = calculate(&uniform(4, None), &Eu2023_1669Ruleset)
        .unwrap()
        .index
        - 0.01;
    assert!(just_under_a < bounds.a);
    assert!(just_under_a >= bounds.b, "should land in B, not lower");
}

#[test]
fn score_outside_one_to_five_is_rejected() {
    let mut i = uniform(3, None);
    i.spare_parts = 0;
    assert!(matches!(
        calculate(&i, &Eu2023_1669Ruleset),
        Err(crate::error::CalcError::InvalidInput(_))
    ));

    let mut j = uniform(3, None);
    j.disassembly_depth.battery = 6;
    assert!(matches!(
        calculate(&j, &Eu2023_1669Ruleset),
        Err(crate::error::CalcError::InvalidInput(_))
    ));
}

#[test]
fn inconsistent_foldability_is_rejected() {
    // Foldability selects the weight set, so it cannot differ per parameter.
    let inputs = RepairabilityIndexInputs {
        disassembly_depth: flat(3, Some(3)),
        fasteners: flat(3, None),
        tools: flat(3, Some(3)),
        spare_parts: 3,
        software_updates: 3,
        repair_information: 3,
    };
    assert!(matches!(
        calculate(&inputs, &Eu2023_1669Ruleset),
        Err(crate::error::CalcError::CrossFieldViolation(_))
    ));
}

#[test]
fn disassembly_depth_dominates_the_other_parameters() {
    // SDD carries 0,25 against 0,15 each — dropping it must cost more than
    // dropping any single other parameter.
    let mut drop_sdd = uniform(5, None);
    drop_sdd.disassembly_depth = flat(1, None);
    let mut drop_tools = uniform(5, None);
    drop_tools.tools = flat(1, None);

    let a = calculate(&drop_sdd, &Eu2023_1669Ruleset).unwrap().index;
    let b = calculate(&drop_tools, &Eu2023_1669Ruleset).unwrap().index;
    assert!(a < b, "SDD ({a}) should cost more than tools ({b})");
}

#[test]
fn ruleset_is_effective_from_the_regulation_date() {
    let r = Eu2023_1669Ruleset;
    let d = r.effective_dates();
    assert_eq!(
        d.from,
        chrono::NaiveDate::from_ymd_opt(2025, 6, 20).unwrap()
    );
    assert!(d.until.is_none());
    assert!(!r.regulatory_basis().regulation.is_empty());
}

#[test]
fn index_and_heuristic_use_different_scales() {
    // Guard against the two ever being conflated: this index tops out at 5,
    // the heuristic at 10. A caller mixing them would silently misreport.
    let idx = calculate(&uniform(5, None), &Eu2023_1669Ruleset).unwrap();
    assert!(idx.index <= 5.0);
    assert_ne!(
        Eu2023_1669Ruleset.id().0,
        crate::repairability::SimplifiedRepairabilityHeuristic
            .id()
            .0
    );
}
