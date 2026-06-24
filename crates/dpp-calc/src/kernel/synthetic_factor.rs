//! Test-only synthetic [`FactorProvider`] returning hardcoded factors.
//!
//! The whole module is compiled only under `cfg(test)` or the
//! `synthetic-factors` feature, so it can never ship in a binary that makes
//! compliance claims.

use super::error::CalcError;
use super::factor::FactorProvider;

/// Test-only synthetic provider that returns hardcoded factors.
///
/// Values are **NOT** real LCI data. Use only in unit tests and golden-vector
/// harnesses — never ship in a binary that makes compliance claims.
pub struct SyntheticFactorProvider {
    entries: std::collections::HashMap<String, f64>,
    table_hash: String,
}

impl SyntheticFactorProvider {
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, f64)>) -> Self {
        use sha2::{Digest, Sha256};

        let entries: std::collections::HashMap<String, f64> =
            entries.into_iter().map(|(k, v)| (k.into(), v)).collect();

        // Sort for deterministic hashing.
        let mut sorted: Vec<(&str, f64)> = entries.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        sorted.sort_by_key(|(k, _)| *k);

        let json = serde_json::to_vec(&sorted).unwrap_or_default();
        let table_hash = hex::encode(Sha256::digest(&json));

        Self {
            entries,
            table_hash,
        }
    }
}

impl FactorProvider for SyntheticFactorProvider {
    fn dataset_id(&self) -> &str {
        "synthetic-test"
    }
    fn dataset_version(&self) -> &str {
        "0.0.0"
    }
    fn gwp100(&self, activity_uuid: &str) -> Result<f64, CalcError> {
        self.entries
            .get(activity_uuid)
            .copied()
            .ok_or_else(|| CalcError::FactorNotFound(activity_uuid.to_owned()))
    }
    fn table_hash(&self) -> &str {
        &self.table_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_known_factor_and_stable_metadata() {
        let provider =
            SyntheticFactorProvider::new([("aluminium-primary", 16.5), ("steel-eaf", 0.8)]);
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
}
