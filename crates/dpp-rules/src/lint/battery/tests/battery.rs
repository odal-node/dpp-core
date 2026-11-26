//! Tests of the `battery` module.

use super::super::*;

fn base_input() -> BatteryLintInput<'static> {
    BatteryLintInput {
        nominal_voltage_v: 3.7,
        nominal_capacity_ah: 10.0,
        rated_energy_wh: Some(37.0),
        rated_capacity_kwh: Some(0.037),
        operating_temp_min_c: Some(-20.0),
        operating_temp_max_c: Some(60.0),
        manufacturing_date_unix: Some(1_000),
        as_of_unix: 2_000,
        cathode_material_pct: &[],
        anode_material_pct: &[],
        electrolyte_material_pct: &[],
    }
}

// ── energy_capacity_mismatch ────────────────────────────────────────────

#[test]
fn energy_capacity_within_tolerance_passes() {
    assert!(energy_capacity_mismatch(&base_input()).is_none());
}

#[test]
fn energy_capacity_far_off_triggers() {
    let mut input = base_input();
    input.rated_energy_wh = Some(200.0); // 37.0 expected, way off
    let finding = energy_capacity_mismatch(&input).unwrap();
    assert_eq!(finding.code, "battery.energy_capacity_mismatch");
    assert_eq!(finding.severity, LintSeverity::Notice);
}

// ── rated_capacity_kwh_wh_mismatch ──────────────────────────────────────

#[test]
fn capacity_unit_consistent_passes() {
    assert!(rated_capacity_kwh_wh_mismatch(&base_input()).is_none());
}

#[test]
fn capacity_unit_mismatch_triggers() {
    let mut input = base_input();
    input.rated_capacity_kwh = Some(1.0); // 1000 Wh expected, declared 37 Wh
    let finding = rated_capacity_kwh_wh_mismatch(&input).unwrap();
    assert_eq!(finding.code, "battery.rated_capacity_kwh_wh_mismatch");
    assert_eq!(finding.severity, LintSeverity::Warning);
}

// ── material_composition_sums ───────────────────────────────────────────

#[test]
fn material_sums_near_100_pass() {
    let mut input = base_input();
    input.cathode_material_pct = &[60.0, 40.0];
    assert!(material_composition_sums(&input).is_empty());
}

#[test]
fn material_sum_off_triggers() {
    let mut input = base_input();
    input.cathode_material_pct = &[60.0, 20.0]; // sums to 80
    let findings = material_composition_sums(&input);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].field, "cathodeMaterial");
}

#[test]
fn empty_material_lists_never_trigger() {
    assert!(material_composition_sums(&base_input()).is_empty());
}

// ── manufacturing_date_in_future ────────────────────────────────────────

#[test]
fn manufacturing_date_in_past_passes() {
    assert!(manufacturing_date_in_future(&base_input()).is_none());
}

#[test]
fn manufacturing_date_in_future_triggers() {
    let mut input = base_input();
    input.manufacturing_date_unix = Some(3_000); // as_of is 2_000
    let finding = manufacturing_date_in_future(&input).unwrap();
    assert_eq!(finding.code, "battery.manufacturing_date_in_future");
}

// ── operating_temp_absurd_range ─────────────────────────────────────────

#[test]
fn plausible_temp_range_passes() {
    assert!(operating_temp_absurd_range(&base_input()).is_empty());
}

#[test]
fn absurd_temp_range_triggers_both_bounds() {
    let mut input = base_input();
    input.operating_temp_min_c = Some(-200.0);
    input.operating_temp_max_c = Some(500.0);
    let findings = operating_temp_absurd_range(&input);
    assert_eq!(findings.len(), 2);
}

// ── lint_battery aggregator ─────────────────────────────────────────────

#[test]
fn clean_input_produces_no_findings() {
    assert!(lint_battery(&base_input()).is_empty());
}
