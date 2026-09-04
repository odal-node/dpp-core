//! Proof-of-calculation receipt — auditable envelope for every calculator result.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::clock::AssessmentClock;
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
#[serde(rename_all = "camelCase")]
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
    /// SHA-256 of the JCS (RFC 8785) canonical JSON of the parameter set this
    /// calculation actually used.
    ///
    /// `ruleset_id` and `ruleset_version` say *which rule*; this says *which
    /// numbers*. Without it two receipts citing the same id and version,
    /// computed by builds carrying different thresholds, are indistinguishable —
    /// and since a version is a string a human maintains, that is not a remote
    /// failure. Always populated, whether or not a bundle was involved: the
    /// baseline case is the one that had no other evidence.
    ///
    /// Empty string until populated via
    /// [`for_ruleset`](CalculationReceipt::for_ruleset), which is the only
    /// constructor that has a ruleset to read it from.
    pub ruleset_content_sha256: String,
    /// Version of the signed Compliance-Current bundle that filled this
    /// ruleset's parameters. `None` when the ruleset came from the built-in
    /// baseline (no signed bundle involved).
    pub bundle_version: Option<String>,
    /// The signed bundle manifest's `contentSha256` — the bytes the publisher
    /// committed to. `None` for the built-in baseline.
    ///
    /// Travels with `bundle_version` because either alone is weak: a version is
    /// a label the publisher chose and may reuse, and a hash with no version
    /// does not say which release to go and fetch.
    pub bundle_content_sha256: Option<String>,
    /// Identifier of the factor dataset (empty if no factor provider was used).
    pub factor_dataset_id: String,
    /// Version of the factor dataset (empty if no factor provider was used).
    pub factor_dataset_version: String,
    /// SHA-256 of the full factor table at calculation time.
    /// `None` when the calculation did not use a `FactorProvider`.
    pub factor_set_hash: Option<String>,
    /// The date whose law this calculation was performed against — the
    /// product's regulated triggering event, not the day it was computed.
    ///
    /// Without this an auditor can see *which* ruleset was cited but not
    /// whether it was the right one to cite, because ruleset selection is a
    /// function of this date. It is the difference between a receipt that can
    /// be re-verified and one that can only be re-read.
    pub assessed_as_of: NaiveDate,
    /// UTC timestamp when the calculation ran.
    pub computed_at: DateTime<Utc>,
    /// JWS signature produced by the vault/engine after calculation.
    /// `None` until the caller calls [`seal_with_jws`](CalculationReceipt::seal_with_jws).
    pub jws: Option<String>,
}

impl CalculationReceipt {
    /// Both timestamps come from `clock` — the receipt never reads the wall
    /// clock itself, so replaying a stored calculation reproduces its dates
    /// exactly rather than stamping today's.
    ///
    /// Crate-private, because it cannot populate `ruleset_content_sha256` — it
    /// takes an id and a version string, not the ruleset those came from. A
    /// receipt built this way would carry an empty parameter hash that *looks*
    /// present, which is worse than not having the field. Callers outside this
    /// crate go through [`for_ruleset`](Self::for_ruleset), which the doc
    /// already told them to use.
    pub(crate) fn new(
        input_hash: impl Into<String>,
        ruleset_id: impl Into<String>,
        ruleset_version: impl Into<String>,
        clock: AssessmentClock,
    ) -> Self {
        Self {
            receipt_id: Uuid::now_v7(),
            input_hash: input_hash.into(),
            output_hash: String::new(),
            ruleset_id: ruleset_id.into(),
            ruleset_version: ruleset_version.into(),
            ruleset_content_sha256: String::new(),
            bundle_version: None,
            bundle_content_sha256: None,
            factor_dataset_id: String::new(),
            factor_dataset_version: String::new(),
            factor_set_hash: None,
            assessed_as_of: clock.law_in_force_on,
            computed_at: clock.computed_at,
            jws: None,
        }
    }

    /// Build the receipt for a `calculate()` call: hashes `inputs`, cites
    /// `ruleset`'s id/version, and attaches `output_hash`. The one-liner every
    /// calculator's `calculate()` should use instead of hand-assembling
    /// `CalculationReceipt::new(...).with_output_hash(...)` — see
    /// `co2e::calculator::calculate` / `repairability::calculator::calculate`.
    /// # Provenance is read here, and only here
    ///
    /// The parameter hash and the bundle fields all come off the same
    /// `ruleset`, so a receipt cannot name a bundle while its hash still
    /// describes the baseline. There is deliberately no setter for any of the
    /// three: `with_bundle_version` used to exist and could be called on a
    /// receipt whose parameters had never been filled, which is exactly the
    /// inconsistency these fields are meant to rule out.
    pub fn for_ruleset<T: Serialize>(
        inputs: &T,
        ruleset: &dyn Ruleset,
        clock: AssessmentClock,
        output_hash: impl Into<String>,
    ) -> Result<Self, CalcError> {
        let mut receipt = Self::new(
            input_hash(inputs)?,
            ruleset.id().0,
            ruleset.version().0,
            clock,
        )
        .with_output_hash(output_hash);

        receipt.ruleset_content_sha256 = ruleset.parameters().content_sha256()?;

        if let Some(provenance) = ruleset.bundle_provenance() {
            receipt.bundle_version = Some(provenance.bundle_version.clone());
            receipt.bundle_content_sha256 = Some(provenance.content_sha256.clone());
        }

        Ok(receipt)
    }

    /// Bind the numeric output values to this receipt.
    pub fn with_output_hash(mut self, hash: impl Into<String>) -> Self {
        self.output_hash = hash.into();
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
    ///
    /// # What is reproducible, and what is not
    ///
    /// These bytes include `receipt_id`, which is a UUIDv7 minted from the wall
    /// clock — so **re-running a calculation does not reproduce these bytes**,
    /// and two receipts for the same determination will not carry the same
    /// signature.
    ///
    /// That is not what an auditor checks. Re-verification compares
    /// `input_hash`, `output_hash`, `ruleset_id`, `ruleset_version`,
    /// `assessed_as_of` and the factor-dataset fields — all of which are
    /// functions of the determination and reproduce exactly. The signature
    /// attests that *this* receipt was issued by the operator, not that the
    /// determination is the only one that could have been issued.
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
    use chrono::NaiveDate;

    fn test_clock() -> AssessmentClock {
        AssessmentClock::placed_on(NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"))
    }
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
        let receipt = CalculationReceipt::new("in-hash", "ruleset-x", "1.0.0", test_clock())
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

    /// The bundle fields default to absent and survive a JSON round trip.
    ///
    /// This asserted `json["bundle_version"]` before, which is not a key this
    /// struct ever emits — it is `rename_all = "camelCase"`, so the key is
    /// `bundleVersion`, and indexing a `serde_json::Value` with a missing key
    /// yields `Null`. The assertion therefore passed no matter what the field
    /// serialised as, and the test's name promised a round trip it never
    /// performed. Both halves are checked here instead.
    #[test]
    fn bundle_fields_default_to_absent_and_round_trip() {
        let receipt = CalculationReceipt::new("in", "r", "1.0.0", test_clock());
        assert_eq!(receipt.bundle_version, None);
        assert_eq!(receipt.bundle_content_sha256, None);

        let json = serde_json::to_value(&receipt).unwrap();
        assert!(
            json.get("bundle_version").is_none(),
            "snake_case key must not appear — the struct is camelCase"
        );
        assert_eq!(json["bundleVersion"], serde_json::Value::Null);
        assert_eq!(json["bundleContentSha256"], serde_json::Value::Null);

        let back: CalculationReceipt = serde_json::from_value(json).unwrap();
        assert_eq!(back.bundle_version, None);
        assert_eq!(back.bundle_content_sha256, None);
        assert_eq!(back.receipt_id, receipt.receipt_id);
    }

    #[test]
    fn canonical_bytes_exclude_the_jws_field() {
        let sealed = CalculationReceipt::new("in", "r", "1.0.0", test_clock())
            .seal_with_jws("secret".to_owned());
        let bytes = sealed.canonical_bytes_for_signing().unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            !text.contains("secret"),
            "jws must be excluded from the signing payload"
        );
        assert!(text.contains("in"));
    }
}
