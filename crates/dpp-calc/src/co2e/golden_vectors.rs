//! Golden-vector regression tests for the cradle-to-gate CO₂e calculator.

use super::*;
use crate::error::CalcError;
use crate::ruleset::Ruleset; // for `.regulatory_basis()` on the concrete ruleset

fn inputs() -> Co2eInputs {
    Co2eInputs {
        materials: vec![
            MaterialFootprint {
                mass_kg: 0.5,
                emission_factor_kg_co2e_per_kg: 8.0,
            },
            MaterialFootprint {
                mass_kg: 0.2,
                emission_factor_kg_co2e_per_kg: 3.0,
            },
        ],
        energy_kwh: 1.5,
        grid_factor_kg_co2e_per_kwh: 0.4,
    }
}

#[test]
fn sums_materials_and_energy() {
    let r = calculate(&inputs(), &CradleToGateRuleset).unwrap();
    // 0.5×8 + 0.2×3 = 4.6; 1.5×0.4 = 0.6
    assert!((r.material_co2e_kg - 4.6).abs() < 1e-9);
    assert!((r.energy_co2e_kg - 0.6).abs() < 1e-9);
    assert!((r.total_co2e_kg - 5.2).abs() < 1e-9);
}

#[test]
fn material_breakdown_matches_total() {
    let r = calculate(&inputs(), &CradleToGateRuleset).unwrap();
    assert_eq!(r.material_breakdown.len(), 2);
    assert!((r.material_breakdown[0].co2e_kg - 4.0).abs() < 1e-9);
    assert!((r.material_breakdown[1].co2e_kg - 0.6).abs() < 1e-9);
    let sum: f64 = r.material_breakdown.iter().map(|l| l.co2e_kg).sum();
    assert!((sum - r.material_co2e_kg).abs() < 1e-9);
}

#[test]
fn declared_stages_are_raw_materials_and_production() {
    let r = calculate(&inputs(), &CradleToGateRuleset).unwrap();
    assert_eq!(
        r.declared_stages,
        vec![LifecycleStage::RawMaterials, LifecycleStage::Production]
    );
}

#[test]
fn empty_bom_is_energy_only() {
    let r = calculate(
        &Co2eInputs {
            materials: vec![],
            energy_kwh: 2.0,
            grid_factor_kg_co2e_per_kwh: 0.5,
        },
        &CradleToGateRuleset,
    )
    .unwrap();
    assert!((r.total_co2e_kg - 1.0).abs() < 1e-9);
    assert!(r.material_breakdown.is_empty());
}

#[test]
fn negative_mass_is_rejected() {
    let err = calculate(
        &Co2eInputs {
            materials: vec![MaterialFootprint {
                mass_kg: -5.0,
                emission_factor_kg_co2e_per_kg: 8.0,
            }],
            energy_kwh: 1.0,
            grid_factor_kg_co2e_per_kwh: 0.4,
        },
        &CradleToGateRuleset,
    );
    assert!(matches!(err, Err(CalcError::InvalidInput(_))));
}

#[test]
fn nan_energy_is_rejected() {
    let err = calculate(
        &Co2eInputs {
            materials: vec![],
            energy_kwh: f64::NAN,
            grid_factor_kg_co2e_per_kwh: 0.4,
        },
        &CradleToGateRuleset,
    );
    assert!(matches!(err, Err(CalcError::InvalidInput(_))));
}

#[test]
fn receipt_binds_to_inputs_and_outputs() {
    let r = calculate(&inputs(), &CradleToGateRuleset).unwrap();
    assert_eq!(r.receipt.ruleset_id, "co2e-cradle-to-gate");
    assert!(!r.receipt.input_hash.is_empty());
    assert!(!r.receipt.output_hash.is_empty());
    // Same inputs produce the same hashes.
    let r2 = calculate(&inputs(), &CradleToGateRuleset).unwrap();
    assert_eq!(r.receipt.input_hash, r2.receipt.input_hash);
    assert_eq!(r.receipt.output_hash, r2.receipt.output_hash);
}

#[test]
fn receipt_can_be_signed_externally() {
    let r = calculate(&inputs(), &CradleToGateRuleset).unwrap();
    let bytes = r.receipt.canonical_bytes_for_signing().unwrap();
    assert!(!bytes.is_empty());
    // Seal it (simulating vault signing).
    let sealed = r.receipt.seal_with_jws("fake.jws.token".into());
    assert_eq!(sealed.jws.as_deref(), Some("fake.jws.token"));
}

#[test]
fn cradle_to_gate_ruleset_has_regulatory_basis() {
    let b = CradleToGateRuleset.regulatory_basis();
    assert!(!b.regulation.is_empty());
    assert!(!b.article.is_empty());
}
