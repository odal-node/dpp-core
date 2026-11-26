//! Tests of the `textile` module.

use super::super::*;

fn base_input() -> TextileLintInput<'static> {
    TextileLintInput {
        durability_score: Some(5.0),
        expected_wash_cycles: Some(50),
        repair_count: None,
        repair_history_url: None,
        prior_use_cycles: None,
        reuse_condition: None,
        repair_score: Some(5.0),
        disassembly_instructions: None,
        spare_parts_available: None,
        microplastic_shedding_mg_per_wash: None,
        fibres: &["cotton", "polyester"],
    }
}

// ── durability_wash_cycles_mismatch ─────────────────────────────────────

#[test]
fn durability_wash_cycles_consistent_passes() {
    assert!(durability_wash_cycles_mismatch(&base_input()).is_none());
}

#[test]
fn high_durability_low_cycles_triggers() {
    let mut input = base_input();
    input.durability_score = Some(9.0);
    input.expected_wash_cycles = Some(5);
    let finding = durability_wash_cycles_mismatch(&input).unwrap();
    assert_eq!(finding.code, "textile.durability_wash_cycles_mismatch");
}

// ── repair_count_without_history ────────────────────────────────────────

#[test]
fn no_repairs_passes() {
    assert!(repair_count_without_history(&base_input()).is_none());
}

#[test]
fn repairs_without_url_triggers() {
    let mut input = base_input();
    input.repair_count = Some(3);
    let finding = repair_count_without_history(&input).unwrap();
    assert_eq!(finding.code, "textile.repair_count_without_history");
}

#[test]
fn repairs_with_url_passes() {
    let mut input = base_input();
    input.repair_count = Some(3);
    input.repair_history_url = Some("https://example.com/repairs/123");
    assert!(repair_count_without_history(&input).is_none());
}

// ── prior_use_without_reuse_condition ───────────────────────────────────

#[test]
fn new_item_passes() {
    assert!(prior_use_without_reuse_condition(&base_input()).is_none());
}

#[test]
fn prior_use_without_condition_triggers() {
    let mut input = base_input();
    input.prior_use_cycles = Some(2);
    let finding = prior_use_without_reuse_condition(&input).unwrap();
    assert_eq!(finding.code, "textile.prior_use_without_reuse_condition");
}

// ── repair_score_high_without_support ───────────────────────────────────

#[test]
fn moderate_repair_score_passes() {
    assert!(repair_score_high_without_support(&base_input()).is_none());
}

#[test]
fn high_repair_score_without_support_triggers() {
    let mut input = base_input();
    input.repair_score = Some(9.0);
    let finding = repair_score_high_without_support(&input).unwrap();
    assert_eq!(finding.code, "textile.repair_score_high_without_support");
}

#[test]
fn high_repair_score_with_spare_parts_passes() {
    let mut input = base_input();
    input.repair_score = Some(9.0);
    input.spare_parts_available = Some(true);
    assert!(repair_score_high_without_support(&input).is_none());
}

// ── microplastic_shedding_without_synthetic_fibre ───────────────────────

#[test]
fn shedding_with_synthetic_fibre_passes() {
    let mut input = base_input();
    input.microplastic_shedding_mg_per_wash = Some(12.0); // fibres include polyester
    assert!(microplastic_shedding_without_synthetic_fibre(&input).is_none());
}

#[test]
fn shedding_with_only_natural_fibres_triggers() {
    let mut input = base_input();
    input.fibres = &["cotton", "wool"];
    input.microplastic_shedding_mg_per_wash = Some(12.0);
    let finding = microplastic_shedding_without_synthetic_fibre(&input).unwrap();
    assert_eq!(
        finding.code,
        "textile.microplastic_shedding_without_synthetic_fibre"
    );
}

#[test]
fn no_shedding_declared_passes() {
    let mut input = base_input();
    input.fibres = &["cotton"];
    assert!(microplastic_shedding_without_synthetic_fibre(&input).is_none());
}

// ── lint_textile aggregator ─────────────────────────────────────────────

#[test]
fn clean_input_produces_no_findings() {
    assert!(lint_textile(&base_input()).is_empty());
}
