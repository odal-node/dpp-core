//! Proof-of-calculation receipt — auditable envelope for every calculator result.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::CalcError;
use super::ruleset::Ruleset;

// Re-export the JCS hashing helpers so callers keep using `receipt::jcs_hash` /
// `receipt::input_hash` — they are split into `hashing.rs` for readability but
// belong to the same proof-of-calculation surface.
pub use super::hashing::{input_hash, jcs_hash};

/// Proof-of-calculation envelope emitted by every calculator function.
///
/// Carries enough information to reproduce or audit the result: both inputs
/// and numeric outputs are JCS-hashed (RFC 8785) so an auditor can verify the
/// same inputs produce the same outputs, and the exact ruleset + factor dataset
/// versions are recorded. The receipt may be signed by the vault via
/// [`seal_with_jws`](CalculationReceipt::seal_with_jws) after calling
/// [`canonical_bytes_for_signing`](CalculationReceipt::canonical_bytes_for_signing).
///
/// Intended to be stored alongside the computed value in the proof-bound store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationReceipt {
    /// Unique receipt identifier (UUIDv7, time-sortable).
    pub receipt_id: Uuid,
    /// SHA-256 of the JCS (RFC 8785) canonical JSON of the calculator inputs.
    pub input_hash: String,
    /// SHA-256 of the JCS (RFC 8785) canonical JSON of the numeric output values.
    /// Empty string until populated via [`with_output_hash`](CalculationReceipt::with_output_hash).
    pub output_hash: String,
    /// Machine-readable identifier of the ruleset used.
    pub ruleset_id: String,
    /// Version of the ruleset (semver-shaped string).
    pub ruleset_version: String,
    /// Version of the signed Compliance-Current bundle that delivered this
    /// ruleset. `None` when the ruleset came from the built-in baseline
    /// (no signed bundle involved).
    pub bundle_version: Option<String>,
    /// Identifier of the factor dataset (empty if no factor provider was used).
    pub factor_dataset_id: String,
    /// Version of the factor dataset (empty if no factor provider was used).
    pub factor_dataset_version: String,
    /// SHA-256 of the full factor table at calculation time.
    /// `None` when the calculation did not use a `FactorProvider`.
    pub factor_set_hash: Option<String>,
    /// UTC timestamp when the calculation ran.
    pub computed_at: DateTime<Utc>,
    /// JWS signature produced by the vault/engine after calculation.
    /// `None` until the caller calls [`seal_with_jws`](CalculationReceipt::seal_with_jws).
    pub jws: Option<String>,
}

impl CalculationReceipt {
    pub fn new(
        input_hash: impl Into<String>,
        ruleset_id: impl Into<String>,
        ruleset_version: impl Into<String>,
    ) -> Self {
        Self {
            receipt_id: Uuid::now_v7(),
            input_hash: input_hash.into(),
            output_hash: String::new(),
            ruleset_id: ruleset_id.into(),
            ruleset_version: ruleset_version.into(),
            bundle_version: None,
            factor_dataset_id: String::new(),
            factor_dataset_version: String::new(),
            factor_set_hash: None,
            computed_at: Utc::now(),
            jws: None,
        }
    }

    /// Build the receipt for a `calculate()` call: hashes `inputs`, cites
    /// `ruleset`'s id/version, and attaches `output_hash`. The one-liner every
    /// calculator's `calculate()` should use instead of hand-assembling
    /// `CalculationReceipt::new(...).with_output_hash(...)` — see
    /// `co2e::calculator::calculate` / `repairability::calculator::calculate`.
    pub fn for_ruleset<T: Serialize>(
        inputs: &T,
        ruleset: &dyn Ruleset,
        output_hash: impl Into<String>,
    ) -> Result<Self, CalcError> {
        Ok(
            Self::new(input_hash(inputs)?, ruleset.id().0, ruleset.version().0)
                .with_output_hash(output_hash),
        )
    }

    /// Bind the numeric output values to this receipt.
    pub fn with_output_hash(mut self, hash: impl Into<String>) -> Self {
        self.output_hash = hash.into();
        self
    }

    /// Stamp the signed Compliance-Current bundle version that delivered this
    /// ruleset. Leave unset (`None`) for the built-in baseline rulesets.
    pub fn with_bundle_version(mut self, bundle_version: impl Into<String>) -> Self {
        self.bundle_version = Some(bundle_version.into());
        self
    }

    /// Attach factor-provider provenance to this receipt.
    pub fn with_factor_provider(mut self, provider: &dyn super::factor::FactorProvider) -> Self {
        self.factor_dataset_id = provider.dataset_id().to_owned();
        self.factor_dataset_version = provider.dataset_version().to_owned();
        self.factor_set_hash = Some(provider.table_hash().to_owned());
        self
    }

    /// Attach a JWS signature produced by the external signing infrastructure.
    /// Call after [`canonical_bytes_for_signing`](CalculationReceipt::canonical_bytes_for_signing)
    /// to avoid signing the jws field itself.
    pub fn seal_with_jws(mut self, jws: String) -> Self {
        self.jws = Some(jws);
        self
    }

    /// JCS-canonical bytes of this receipt without the `jws` field.
    ///
    /// Pass these bytes to the vault's signing infrastructure, then call
    /// [`seal_with_jws`](CalculationReceipt::seal_with_jws) with the resulting
    /// JWS to produce the final sealed receipt.
    pub fn canonical_bytes_for_signing(&self) -> Result<Vec<u8>, CalcError> {
        let mut v =
            serde_json::to_value(self).map_err(|e| CalcError::CanonicalizeError(e.to_string()))?;
        if let Some(obj) = v.as_object_mut() {
            obj.remove("jws");
        }
        serde_jcs::to_vec(&v).map_err(|e| CalcError::CanonicalizeError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factor::FactorProvider;

    struct DummyProvider;
    impl FactorProvider for DummyProvider {
        fn dataset_id(&self) -> &str {
            "dummy-ds"
        }
        fn dataset_version(&self) -> &str {
            "1.2.3"
        }
        fn gwp100(&self, _activity_uuid: &str) -> Result<f64, CalcError> {
            Ok(1.0)
        }
        fn table_hash(&self) -> &str {
            "deadbeef"
        }
    }

    #[test]
    fn builder_records_output_factor_and_jws() {
        let receipt = CalculationReceipt::new("in-hash", "ruleset-x", "1.0.0")
            .with_output_hash("out-hash")
            .with_factor_provider(&DummyProvider)
            .seal_with_jws("jws-token".to_owned());

        assert_eq!(receipt.input_hash, "in-hash");
        assert_eq!(receipt.output_hash, "out-hash");
        assert_eq!(receipt.ruleset_id, "ruleset-x");
        assert_eq!(receipt.ruleset_version, "1.0.0");
        assert_eq!(receipt.factor_dataset_id, "dummy-ds");
        assert_eq!(receipt.factor_dataset_version, "1.2.3");
        assert_eq!(receipt.factor_set_hash.as_deref(), Some("deadbeef"));
        assert_eq!(receipt.jws.as_deref(), Some("jws-token"));
    }

    #[test]
    fn bundle_version_defaults_to_none_and_round_trips() {
        let receipt = CalculationReceipt::new("in", "r", "1.0.0");
        assert_eq!(receipt.bundle_version, None);

        let json = serde_json::to_value(&receipt).unwrap();
        assert_eq!(json["bundle_version"], serde_json::Value::Null);

        let stamped = receipt.with_bundle_version("bundle-2026.07");
        assert_eq!(stamped.bundle_version.as_deref(), Some("bundle-2026.07"));
    }

    #[test]
    fn canonical_bytes_exclude_the_jws_field() {
        let sealed = CalculationReceipt::new("in", "r", "1.0.0").seal_with_jws("secret".to_owned());
        let bytes = sealed.canonical_bytes_for_signing().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            !text.contains("secret"),
            "jws must be excluded from the signing payload"
        );
        assert!(text.contains("in"));
    }
}
