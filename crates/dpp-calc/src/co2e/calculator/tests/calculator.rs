//! Tests of the `calculator` module.

use super::super::*;
use crate::co2e::parameters::MaterialFootprint;
use crate::co2e::thresholds::CradleToGateRuleset;
use chrono::NaiveDate;

/// A clock whose governing-law date sits inside CradleToGateRuleset's
/// effective period (open from 2021-01-01).
fn in_force() -> AssessmentClock {
    AssessmentClock::placed_on(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
}

fn material(mass: f64, factor: f64) -> MaterialFootprint {
    MaterialFootprint {
        mass_kg: mass,
        emission_factor_kg_co2e_per_kg: factor,
    }
}

#[test]
fn rejects_ruleset_not_yet_in_force() {
    // CradleToGateRuleset is effective from 2021-01-01 — querying a date
    // before that exercises the not-yet-effective path on the real
    // ruleset, with no need for a synthetic future-dated test double.
    let inputs = Co2eInputs {
        materials: vec![material(1.0, 2.0)],
        energy_kwh: 1.0,
        grid_factor_kg_co2e_per_kwh: 0.4,
    };
    let before_effective = AssessmentClock::placed_on(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());
    assert!(matches!(
        calculate(&inputs, &CradleToGateRuleset, before_effective),
        Err(CalcError::RulesetNotYetEffective { .. })
    ));
}

#[test]
fn rejects_overflow_to_non_finite() {
    // Each input is finite and in range, but the product overflows f64.
    let inputs = Co2eInputs {
        materials: vec![material(1e200, 1e200)],
        energy_kwh: 0.0,
        grid_factor_kg_co2e_per_kwh: 0.0,
    };
    assert!(matches!(
        calculate(&inputs, &CradleToGateRuleset, in_force()),
        Err(CalcError::Overflow(_))
    ));
}

#[test]
fn rejects_negative_energy() {
    let inputs = Co2eInputs {
        materials: vec![material(1.0, 2.0)],
        energy_kwh: -1.0,
        grid_factor_kg_co2e_per_kwh: 0.4,
    };
    assert!(matches!(
        calculate(&inputs, &CradleToGateRuleset, in_force()),
        Err(CalcError::InvalidInput(_))
    ));
}

#[test]
fn rejects_non_finite_grid_factor() {
    let inputs = Co2eInputs {
        materials: vec![material(1.0, 2.0)],
        energy_kwh: 1.0,
        grid_factor_kg_co2e_per_kwh: f64::NAN,
    };
    assert!(matches!(
        calculate(&inputs, &CradleToGateRuleset, in_force()),
        Err(CalcError::InvalidInput(_))
    ));
}

#[test]
fn rejects_negative_material_mass() {
    let inputs = Co2eInputs {
        materials: vec![material(-0.5, 2.0)],
        energy_kwh: 1.0,
        grid_factor_kg_co2e_per_kwh: 0.4,
    };
    assert!(matches!(
        calculate(&inputs, &CradleToGateRuleset, in_force()),
        Err(CalcError::InvalidInput(_))
    ));
}

#[test]
fn rejects_negative_emission_factor() {
    let inputs = Co2eInputs {
        materials: vec![material(0.5, -2.0)],
        energy_kwh: 1.0,
        grid_factor_kg_co2e_per_kwh: 0.4,
    };
    assert!(matches!(
        calculate(&inputs, &CradleToGateRuleset, in_force()),
        Err(CalcError::InvalidInput(_))
    ));
}

#[test]
fn accepts_valid_inputs_and_sums_correctly() {
    let inputs = Co2eInputs {
        materials: vec![material(2.0, 3.0), material(1.0, 4.0)],
        energy_kwh: 10.0,
        grid_factor_kg_co2e_per_kwh: 0.5,
    };
    let result = calculate(&inputs, &CradleToGateRuleset, in_force()).unwrap();
    // materials: 2*3 + 1*4 = 10; energy: 10*0.5 = 5; total = 15
    assert!((result.material_co2e_kg - 10.0).abs() < 1e-9);
    assert!((result.energy_co2e_kg - 5.0).abs() < 1e-9);
    assert!((result.total_co2e_kg - 15.0).abs() < 1e-9);
    assert_eq!(result.material_breakdown.len(), 2);
}
