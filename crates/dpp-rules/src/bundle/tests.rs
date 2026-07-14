//! Bundle format + verification tests, driven with a fake [`JwsVerify`] so
//! this crate's own tests never need a real signing key. The real EdDSA path
//! is covered end-to-end where the adapter lives (engine-side).

use std::collections::BTreeMap;

use super::*;

struct AlwaysOk;
impl JwsVerify for AlwaysOk {
    fn verify_eddsa(&self, _jws: &str, _public_key_b64: &str) -> Result<bool, RulesetError> {
        Ok(true)
    }
}

struct AlwaysBad;
impl JwsVerify for AlwaysBad {
    fn verify_eddsa(&self, _jws: &str, _public_key_b64: &str) -> Result<bool, RulesetError> {
        Ok(false)
    }
}

struct AlwaysErr;
impl JwsVerify for AlwaysErr {
    fn verify_eddsa(&self, _jws: &str, _public_key_b64: &str) -> Result<bool, RulesetError> {
        Err(RulesetError::Malformed("verifier exploded".into()))
    }
}

fn manifest(version: &str, content: &serde_json::Value) -> RulesetManifest {
    RulesetManifest {
        bundle_version: version.into(),
        effective_date: chrono::Utc::now(),
        act_citations: vec!["ESPR Art. 25".into()],
        schema_versions: BTreeMap::from([("textile".to_owned(), "2.0.0".to_owned())]),
        content_sha256: content_hash(content).expect("finite test content hashes"),
    }
}

/// A JWS-shaped string whose payload segment decodes to `m`. Header and
/// signature segments are placeholders — `verify_bundle` never parses them
/// itself; that's the injected verifier's job.
fn fake_jws(m: &RulesetManifest) -> String {
    use base64::Engine;
    let payload = serde_json::to_vec(m).expect("manifest serialises");
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
    format!("aGVhZGVy.{payload_b64}.c2ln")
}

fn bundle(version: &str, threshold: i64) -> SignedBundle {
    let content = serde_json::json!({ "textileFibreThreshold": threshold });
    let m = manifest(version, &content);
    SignedBundle {
        manifest_jws: fake_jws(&m),
        content,
    }
}

#[test]
fn valid_bundle_verifies_and_carries_version() {
    let b = bundle("2026-Q3.1", 5);
    let v = verify_bundle(&b, "pubkey", &AlwaysOk).expect("must verify");
    assert_eq!(v.version(), "2026-Q3.1");
    assert_eq!(v.content["textileFibreThreshold"], 5);
}

#[test]
fn bad_signature_is_refused() {
    let b = bundle("2026-Q3.1", 5);
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysBad),
        Err(RulesetError::BadSignature)
    ));
}

#[test]
fn verifier_error_propagates_as_malformed() {
    let b = bundle("2026-Q3.1", 5);
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysErr),
        Err(RulesetError::Malformed(_))
    ));
}

#[test]
fn tampered_content_is_refused() {
    let mut b = bundle("2026-Q3.1", 5);
    // Change the content without updating the signed manifest's hash.
    b.content = serde_json::json!({ "textileFibreThreshold": 999 });
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysOk),
        Err(RulesetError::ContentHashMismatch)
    ));
}

#[test]
fn malformed_jws_missing_payload_segment_is_refused() {
    let b = SignedBundle {
        manifest_jws: "onlyoneseg".into(),
        content: serde_json::json!({}),
    };
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysOk),
        Err(RulesetError::Malformed(_))
    ));
}

#[test]
fn malformed_jws_payload_not_json_is_refused() {
    use base64::Engine;
    let bad_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"not json");
    let b = SignedBundle {
        manifest_jws: format!("aGVhZGVy.{bad_payload}.c2ln"),
        content: serde_json::json!({}),
    };
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysOk),
        Err(RulesetError::Malformed(_))
    ));
}

#[test]
fn content_hash_is_stable_for_key_order() {
    let a = serde_json::json!({ "a": 1, "b": 2 });
    let b = serde_json::json!({ "b": 2, "a": 1 });
    assert_eq!(content_hash(&a).unwrap(), content_hash(&b).unwrap());
}

#[test]
fn content_hash_errors_on_non_finite_float() {
    // serde_json parses a huge exponent to f64::INFINITY; JCS (RFC 8785)
    // rejects non-finite floats. content_hash must return Err, not panic.
    let content: serde_json::Value = serde_json::from_str(r#"{ "x": 1e400 }"#).unwrap();
    assert!(content["x"].as_f64().unwrap().is_infinite());
    assert!(matches!(
        content_hash(&content),
        Err(RulesetError::Malformed(_))
    ));
}

#[test]
fn verify_bundle_errors_on_non_finite_content() {
    // A well-signed bundle whose unauthenticated content holds a non-finite
    // float must fail closed on the integrity step, not panic.
    let content: serde_json::Value = serde_json::from_str(r#"{ "threshold": 1e400 }"#).unwrap();
    let m = manifest("1.0.0", &serde_json::json!({ "threshold": 0 }));
    let bundle = SignedBundle {
        manifest_jws: fake_jws(&m),
        content,
    };
    assert!(matches!(
        verify_bundle(&bundle, "pubkey", &AlwaysOk),
        Err(RulesetError::Malformed(_))
    ));
}
