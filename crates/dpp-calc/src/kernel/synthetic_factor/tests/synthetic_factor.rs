//! Tests of the `synthetic_factor` module.

use super::super::super::error::CalcError;
use super::super::super::factor::FactorProvider;
use super::super::*;

#[test]
fn returns_known_factor_and_stable_metadata() {
    let provider = SyntheticFactorProvider::new([("aluminium-primary", 16.5), ("steel-eaf", 0.8)]);
    assert_eq!(provider.dataset_id(), "synthetic-test");
    assert_eq!(provider.dataset_version(), "0.0.0");
    assert_eq!(provider.gwp100("aluminium-primary").unwrap(), 16.5);
    assert_eq!(provider.gwp100("steel-eaf").unwrap(), 0.8);
    // Table hash is a deterministic 64-char SHA-256 hex digest.
    assert_eq!(provider.table_hash().len(), 64);
}

#[test]
fn unknown_activity_is_factor_not_found() {
    let provider = SyntheticFactorProvider::new([("known", 1.0)]);
    assert!(matches!(
        provider.gwp100("unknown"),
        Err(CalcError::FactorNotFound(_))
    ));
}

#[test]
fn table_hash_is_order_independent() {
    let a = SyntheticFactorProvider::new([("x", 1.0), ("y", 2.0)]);
    let b = SyntheticFactorProvider::new([("y", 2.0), ("x", 1.0)]);
    assert_eq!(a.table_hash(), b.table_hash());
}
