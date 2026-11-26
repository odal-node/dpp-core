//! Tests of the `cfb` module.

use super::super::*;
use crate::factor::SyntheticFactorProvider;

#[test]
fn calculate_cfb_returns_not_implemented() {
    let inputs = CfbInputs {
        model_id: "test-battery".into(),
        battery_chemistry: "NMC".into(),
        nominal_capacity_kwh: 60.0,
    };
    let provider = SyntheticFactorProvider::new(std::iter::empty::<(String, f64)>());
    let err = calculate_cfb(&inputs, &provider).unwrap_err();
    assert!(
        matches!(err, CalcError::NotImplemented { .. }),
        "expected NotImplemented, got {err}"
    );
}
