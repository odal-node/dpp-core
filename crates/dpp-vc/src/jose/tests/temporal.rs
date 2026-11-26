//! Time: `now` is explicit, and `exp` and `nbf` are read only under `Some(now)`.
//!
//! `iat` is never read and has its own test in `verify_tests`. The presentation's own
//! `exp`, and the one of each envelope, are held to the same `now`.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::super::VerifyOptions;
use super::super::{JoseError, verify_credential, verify_presentation};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::at;
use crate::test_support::{
    credential, credential_header, envelope, forge, generated, presentation, presentation_header,
};
use crate::test_support::{public_key, token};

const NOW: i64 = 1_800_000_000;

fn key() -> KeyEntry {
    generated(KeyAlgorithm::Ed25519)
}

fn credential_with(key: &KeyEntry, claim: &str, value: Value) -> String {
    let mut document = credential();
    document[claim] = value;
    forge(key, &credential_header(key), &document)
}

fn verify(token: &str, key: &KeyEntry, now: Option<i64>) -> Result<Value, JoseError> {
    verify_credential(token, &key.public_key(), at(now))
}

// ── exp ───────────────────────────────────────────────────────────────────────

#[test]
fn a_credential_is_valid_until_exp_and_not_at_it() {
    let key = key();
    let token = credential_with(&key, "exp", json!(NOW));

    assert!(verify(&token, &key, Some(NOW - 1)).is_ok());
    assert_eq!(verify(&token, &key, Some(NOW)), Err(JoseError::Expired));
    assert_eq!(verify(&token, &key, Some(NOW + 1)), Err(JoseError::Expired));
}

#[test]
fn exp_may_be_fractional() {
    let key = key();
    let token = credential_with(&key, "exp", json!(1_800_000_000.5));

    assert!(verify(&token, &key, Some(1_800_000_000)).is_ok());
    assert_eq!(
        verify(&token, &key, Some(1_800_000_001)),
        Err(JoseError::Expired)
    );
}

// ── nbf ───────────────────────────────────────────────────────────────────────

#[test]
fn a_credential_is_valid_from_nbf_and_not_before() {
    let key = key();
    let token = credential_with(&key, "nbf", json!(NOW));

    assert_eq!(
        verify(&token, &key, Some(NOW - 1)),
        Err(JoseError::NotYetValid)
    );
    assert!(verify(&token, &key, Some(NOW)).is_ok());
    assert!(verify(&token, &key, Some(NOW + 1)).is_ok());
}

#[test]
fn a_credential_with_both_is_held_to_both() {
    let key = key();
    let mut document = credential();
    document["nbf"] = json!(NOW - 100);
    document["exp"] = json!(NOW + 100);
    let token = forge(&key, &credential_header(&key), &document);

    assert_eq!(
        verify(&token, &key, Some(NOW - 101)),
        Err(JoseError::NotYetValid)
    );
    assert!(verify(&token, &key, Some(NOW)).is_ok());
    assert_eq!(
        verify(&token, &key, Some(NOW + 100)),
        Err(JoseError::Expired)
    );
}

// ── a claim that is not a number ──────────────────────────────────────────────

#[test]
fn a_time_claim_that_is_not_a_number_is_refused_under_some_now() {
    let key = key();
    for claim in ["exp", "nbf"] {
        for value in [
            json!("1900000000"),
            json!("2030-01-01T00:00:00Z"),
            json!(null),
            json!(true),
            json!([1_900_000_000]),
            json!({}),
        ] {
            let token = credential_with(&key, claim, value.clone());
            let refused = verify(&token, &key, Some(NOW)).expect_err("refused");
            assert_eq!(
                refused,
                JoseError::MalformedTemporalClaim {
                    claim: if claim == "exp" { "exp" } else { "nbf" }
                },
                "{claim}: {value}"
            );
        }
    }
}

// ── None ──────────────────────────────────────────────────────────────────────

/// Under `None` nothing about `exp` or `nbf` is read: an expired one, one not yet
/// reached, and one that is not even a number all pass.
#[test]
fn under_none_neither_claim_is_read_or_type_checked() {
    let key = key();
    for (claim, value) in [
        ("exp", json!(1)),
        ("exp", json!("tomorrow")),
        ("exp", json!(null)),
        ("nbf", json!(99_999_999_999_i64)),
        ("nbf", json!("soon")),
        ("nbf", json!([])),
    ] {
        let token = credential_with(&key, claim, value.clone());
        assert!(verify(&token, &key, None).is_ok(), "{claim}: {value}");
    }
}

// ── presentations ─────────────────────────────────────────────────────────────

#[test]
fn the_suites_presentation_has_expired_and_is_refused_only_under_some_now() {
    let key = public_key("p384");
    let file = token("presentation-jose-multiple.txt");

    assert!(verify_presentation(&file, &key, VerifyOptions::new()).is_ok());
    assert!(verify_presentation(&file, &key, VerifyOptions::new().at(1_734_397_449)).is_ok());
    assert_eq!(
        verify_presentation(&file, &key, VerifyOptions::new().at(1_789_000_000)).map(|_| ()),
        Err(JoseError::Expired)
    );
}

#[test]
fn an_envelope_is_held_to_the_same_now() {
    let holder = key();
    let issuer = key();
    let inner = credential_with(&issuer, "exp", json!(NOW));
    let outer = forge(
        &holder,
        &presentation_header(&holder),
        &presentation(vec![envelope(&inner)]),
    );
    let verified =
        verify_presentation(&outer, &holder.public_key(), VerifyOptions::new()).expect("verifies");
    let verify_inner =
        |now| verified.verify_credentials(at(now), &NOT_REQUIRED, |_| Some(issuer.public_key()));

    assert!(verify_inner(Some(NOW - 1)).is_ok());
    assert_eq!(
        verify_inner(Some(NOW)).map(|_| ()),
        Err(JoseError::CredentialRefused {
            index: 0,
            source: Box::new(JoseError::Expired)
        })
    );
    assert!(verify_inner(None).is_ok());
}

/// The outer token is checked completely, time included, before any envelope is read:
/// an expired presentation whose envelope is malformed is refused as expired.
#[test]
fn the_outer_time_is_checked_before_any_envelope() {
    let holder = key();
    let mut document = presentation(vec![json!("not an envelope")]);
    document["exp"] = json!(NOW);
    let outer = forge(&holder, &presentation_header(&holder), &document);

    assert_eq!(
        verify_presentation(&outer, &holder.public_key(), VerifyOptions::new().at(NOW)).map(|_| ()),
        Err(JoseError::Expired)
    );
    assert!(matches!(
        verify_presentation(&outer, &holder.public_key(), VerifyOptions::new()).map(|_| ()),
        Err(JoseError::MalformedEnvelope { index: 0, .. })
    ));
}
