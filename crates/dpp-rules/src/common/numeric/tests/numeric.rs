//! Tests of the `numeric` module.

use super::super::*;

#[test]
fn percentage_in_range_accepts_boundaries() {
    assert!(percentage_in_range(0.0));
    assert!(percentage_in_range(100.0));
    assert!(percentage_in_range(50.0));
}

#[test]
fn percentage_in_range_rejects_out_of_bounds_and_non_finite() {
    assert!(!percentage_in_range(-0.001));
    assert!(!percentage_in_range(100.001));
    assert!(!percentage_in_range(f64::NAN));
    assert!(!percentage_in_range(f64::INFINITY));
}

#[test]
fn sums_to_within_tolerance() {
    let (ok, total) = sums_to([60.0, 40.0], 100.0, 2.0);
    assert!(ok);
    assert!((total - 100.0).abs() < f64::EPSILON);

    let (ok, total) = sums_to([98.5, 1.0], 100.0, 2.0);
    assert!(ok, "99.5 is within ±2 of 100");
    assert!((total - 99.5).abs() < f64::EPSILON);
}

#[test]
fn sums_to_outside_tolerance() {
    let (ok, total) = sums_to([60.0, 30.0], 100.0, 2.0);
    assert!(!ok);
    assert!((total - 90.0).abs() < f64::EPSILON);
}

#[test]
fn sums_to_non_finite_input_is_never_ok() {
    let (ok, total) = sums_to([f64::NAN, 50.0], 100.0, 2.0);
    assert!(!ok);
    assert!(total.is_nan());
}

#[test]
fn sums_to_empty_is_zero() {
    let (ok, total) = sums_to([], 100.0, 2.0);
    assert!(!ok);
    assert_eq!(total, 0.0);
}
