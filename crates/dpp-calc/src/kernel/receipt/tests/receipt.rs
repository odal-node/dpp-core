//! Tests of the `receipt` module.

use super::super::super::clock::AssessmentClock;
use super::super::super::error::CalcError;
use super::super::*;
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
