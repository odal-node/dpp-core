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

/// A fixed clock, so nothing here depends on when it runs.
fn at(y: i32, m: u32, d: u32) -> chrono::DateTime<chrono::Utc> {
    chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, y, m, d, 0, 0, 0).unwrap()
}

/// A policy that accepts anything the signature and hash allow — the shape
/// every pre-existing test was written against, before applicability and
/// currency were checked at all.
fn accepting() -> AcceptancePolicy<'static> {
    AcceptancePolicy {
        now: at(2030, 1, 1),
        in_force: None,
    }
}

fn manifest(version: &str, content: &serde_json::Value) -> RulesetManifest {
    manifest_effective(version, content, at(2026, 7, 1))
}

fn manifest_effective(
    version: &str,
    content: &serde_json::Value,
    effective_date: chrono::DateTime<chrono::Utc>,
) -> RulesetManifest {
    RulesetManifest {
        bundle_version: version.into(),
        effective_date,
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
    let v = verify_bundle(&b, "pubkey", &AlwaysOk, &accepting()).expect("must verify");
    assert_eq!(v.version(), "2026-Q3.1");
    assert_eq!(v.content()["textileFibreThreshold"], 5);
}

#[test]
fn bad_signature_is_refused() {
    let b = bundle("2026-Q3.1", 5);
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysBad, &accepting()),
        Err(RulesetError::BadSignature)
    ));
}

#[test]
fn verifier_error_propagates_as_malformed() {
    let b = bundle("2026-Q3.1", 5);
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysErr, &accepting()),
        Err(RulesetError::Malformed(_))
    ));
}

#[test]
fn tampered_content_is_refused() {
    let mut b = bundle("2026-Q3.1", 5);
    // Change the content without updating the signed manifest's hash.
    b.content = serde_json::json!({ "textileFibreThreshold": 999 });
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysOk, &accepting()),
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
        verify_bundle(&b, "pubkey", &AlwaysOk, &accepting()),
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
        verify_bundle(&b, "pubkey", &AlwaysOk, &accepting()),
        Err(RulesetError::Malformed(_))
    ));
}

#[test]
fn content_hash_is_stable_for_key_order() {
    let a = serde_json::json!({ "a": 1, "b": 2 });
    let b = serde_json::json!({ "b": 2, "a": 1 });
    assert_eq!(content_hash(&a).unwrap(), content_hash(&b).unwrap());
}

/// Tripwire on the upstream guarantee that keeps [`content_hash`]'s error path
/// unreachable in practice. RFC 8785 rejects non-finite floats, so the question
/// is whether one can reach the hasher at all: today it cannot, because
/// `serde_json` refuses to build one — an overflowing literal fails to parse
/// rather than coercing to infinity, and there is no `Number` for a non-finite
/// `f64`.
///
/// The fallible signature stays regardless. This guarantee is upstream's to
/// change (and `serde_json`'s `arbitrary_precision` feature would change it),
/// so the contract must not be relaxed to an infallible one that papers the
/// case over with a panic. If this test ever fails, that error path became
/// reachable and the hazard is live again.
#[test]
fn non_finite_floats_cannot_enter_a_json_value() {
    // Parse side: an overflowing literal is rejected, not coerced to infinity.
    assert!(
        serde_json::from_str::<serde_json::Value>(r#"{ "x": 1e400 }"#).is_err(),
        "overflowing float literal must not parse into a Value"
    );
    // Constructor side: no non-finite f64 can become a JSON number.
    assert!(serde_json::Number::from_f64(f64::INFINITY).is_none());
    assert!(serde_json::Number::from_f64(f64::NAN).is_none());
}

// ── Applicability and currency ────────────────────────────────────────────────
//
// Neither was checked before 2026-08-27, so both of these tests fail against
// the previous implementation rather than merely covering it.

/// A bundle whose rules start after `now` is authentic but not yet applicable.
#[test]
fn a_future_dated_bundle_is_refused_as_not_yet_effective() {
    let content = serde_json::json!({ "textileFibreThreshold": 5 });
    let m = manifest_effective("2027-Q1.1", &content, at(2027, 1, 1));
    let b = SignedBundle {
        manifest_jws: fake_jws(&m),
        content,
    };
    let policy = AcceptancePolicy {
        now: at(2026, 9, 1),
        in_force: None,
    };
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysOk, &policy),
        Err(RulesetError::NotYetEffective { .. })
    ));
}

/// The boundary is inclusive: a bundle effective exactly now is applicable.
#[test]
fn a_bundle_effective_exactly_now_is_applicable() {
    let content = serde_json::json!({ "textileFibreThreshold": 5 });
    let m = manifest_effective("2026-Q4.1", &content, at(2026, 10, 1));
    let b = SignedBundle {
        manifest_jws: fake_jws(&m),
        content,
    };
    let policy = AcceptancePolicy {
        now: at(2026, 10, 1),
        in_force: None,
    };
    assert!(verify_bundle(&b, "pubkey", &AlwaysOk, &policy).is_ok());
}

/// **The rollback case.** An older, perfectly-signed bundle must not displace a
/// newer one — a signature never expires, so without this anyone able to serve
/// bytes could pin a node to superseded rules without forging anything.
#[test]
fn an_older_bundle_is_refused_as_superseded() {
    let in_force = manifest_effective(
        "2026-Q4.1",
        &serde_json::json!({ "textileFibreThreshold": 7 }),
        at(2026, 10, 1),
    );
    let content = serde_json::json!({ "textileFibreThreshold": 5 });
    let old = manifest_effective("2026-Q3.1", &content, at(2026, 7, 1));
    let b = SignedBundle {
        manifest_jws: fake_jws(&old),
        content,
    };
    let policy = AcceptancePolicy {
        now: at(2027, 1, 1),
        in_force: Some(&in_force),
    };
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysOk, &policy),
        Err(RulesetError::Superseded { .. })
    ));
}

/// A republish at the same effective date is a correction, not a rollback.
/// Refusing it would leave no way to ship one.
#[test]
fn a_republish_at_the_same_effective_date_is_accepted() {
    let effective = at(2026, 10, 1);
    let in_force = manifest_effective(
        "2026-Q4.1",
        &serde_json::json!({ "textileFibreThreshold": 7 }),
        effective,
    );
    let content = serde_json::json!({ "textileFibreThreshold": 8 });
    let corrected = manifest_effective("2026-Q4.2", &content, effective);
    let b = SignedBundle {
        manifest_jws: fake_jws(&corrected),
        content,
    };
    let policy = AcceptancePolicy {
        now: at(2027, 1, 1),
        in_force: Some(&in_force),
    };
    let v = verify_bundle(&b, "pubkey", &AlwaysOk, &policy).expect("same-date republish");
    assert_eq!(v.version(), "2026-Q4.2");
}

/// A future-dated bundle reports *that*, not "superseded", even when something
/// older is in force. The two answers tell a caller to do different things:
/// hold and re-offer, versus discard.
#[test]
fn applicability_is_reported_before_currency() {
    let in_force = manifest_effective("2026-Q4.1", &serde_json::json!({}), at(2026, 10, 1));
    let content = serde_json::json!({ "textileFibreThreshold": 9 });
    let future = manifest_effective("2028-Q1.1", &content, at(2028, 1, 1));
    let b = SignedBundle {
        manifest_jws: fake_jws(&future),
        content,
    };
    let policy = AcceptancePolicy {
        now: at(2027, 1, 1),
        in_force: Some(&in_force),
    };
    assert!(matches!(
        verify_bundle(&b, "pubkey", &AlwaysOk, &policy),
        Err(RulesetError::NotYetEffective { .. })
    ));
}

// ── Provenance ────────────────────────────────────────────────────────────────

/// Verification is the only route to `Verified`, and the baseline route says so
/// about itself. The old type claimed holding one was proof of verification
/// while every field was public; this is that claim made checkable.
#[test]
fn provenance_distinguishes_a_verified_bundle_from_a_local_baseline() {
    let b = bundle("2026-Q3.1", 5);
    let verified = verify_bundle(&b, "pubkey", &AlwaysOk, &accepting()).expect("must verify");
    assert_eq!(verified.provenance(), RulesetProvenance::Verified);

    let content = serde_json::json!({});
    let baseline = RulesetAcceptance::unverified_baseline(
        manifest_effective("baseline", &content, at(2026, 1, 1)),
        content,
    );
    assert_eq!(baseline.provenance(), RulesetProvenance::LocalBaseline);
    assert_eq!(baseline.version(), "baseline");
}
