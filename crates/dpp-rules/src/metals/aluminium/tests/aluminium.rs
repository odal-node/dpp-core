//! Tests of the `aluminium` module.

use super::super::*;

#[test]
fn primary_at_threshold_passes() {
    assert!(co2e_within_route_threshold("primary", 10_000.0));
    assert!(co2e_within_route_threshold("primary", 6_000.0));
}

#[test]
fn primary_above_threshold_fails() {
    assert!(!co2e_within_route_threshold("primary", 10_001.0));
}

#[test]
fn secondary_recycled_threshold() {
    assert!(co2e_within_route_threshold("secondary-recycled", 600.0));
    assert!(!co2e_within_route_threshold("secondary-recycled", 1_001.0));
}

#[test]
fn mixed_threshold() {
    assert!(co2e_within_route_threshold("mixed", 5_000.0));
    assert!(!co2e_within_route_threshold("mixed", 5_001.0));
}

#[test]
fn unknown_route_fails() {
    assert!(!co2e_within_route_threshold("unknown", 0.0));
}
