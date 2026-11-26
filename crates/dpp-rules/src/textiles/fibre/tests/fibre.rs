//! Tests of the `fibre` module.

use super::super::*;

fn fibre(name: &str, pct: f64) -> FibreInput<'_> {
    FibreInput {
        fibre: name,
        pct,
        country_of_origin: None,
    }
}

#[test]
fn fibre_sum_within_tolerance() {
    assert!(fibre_sum_ok(&[60.0, 40.0]));
    assert!(fibre_sum_ok(&[98.5, 1.0])); // 99.5 within ±2
    assert!(!fibre_sum_ok(&[60.0, 30.0])); // 90
    assert!(!fibre_sum_ok(&[])); // empty → 0
}

#[test]
fn fibre_composition_valid_passes() {
    assert!(validate_fibre_composition(&[fibre("cotton", 60.0), fibre("polyester", 40.0)]).is_ok());
}

#[test]
fn fibre_sum_invalid_message_has_total() {
    let err =
        validate_fibre_composition(&[fibre("cotton", 60.0), fibre("polyester", 30.0)]).unwrap_err();
    assert!(err.contains("90.0"), "unexpected: {err}");
}

#[test]
fn fibre_invalid_country_rejected() {
    let entry = FibreInput {
        fibre: "cotton",
        pct: 100.0,
        country_of_origin: Some("india"),
    };
    let err = validate_fibre_composition(&[entry]).unwrap_err();
    assert!(err.contains("country_of_origin"), "unexpected: {err}");
}

#[test]
fn fibre_empty_rejected() {
    assert!(validate_fibre_composition(&[]).is_err());
}

#[test]
fn nan_pct_rejected() {
    let err = validate_fibre_composition(&[fibre("cotton", f64::NAN)]).unwrap_err();
    assert!(
        err.contains("NaN") || err.contains("finite"),
        "unexpected: {err}"
    );
}

#[test]
fn infinity_pct_rejected() {
    let err = validate_fibre_composition(&[fibre("cotton", f64::INFINITY)]).unwrap_err();
    assert!(
        err.contains("inf") || err.contains("finite"),
        "unexpected: {err}"
    );
}
