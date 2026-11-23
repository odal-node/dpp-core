//! The Status List Token and the check of section 8.3.

use std::time::Duration;

use base64::Engine;
use chrono::TimeZone;
use serde_json::{Value, json};

use dpp_crypto::keystore::KeyEntry;

use super::test_fixtures::*;
use super::{Status, StatusListToken, TYP, TokenStatusList, TokenStatusListError};
use crate::test_support::{EVERY_ALGORITHM, forge, generated};

fn key() -> KeyEntry {
    generated(dpp_crypto::jws::KeyAlgorithm::Ed25519)
}

fn verified(key: &KeyEntry, claims: &Value) -> Result<StatusListToken, TokenStatusListError> {
    StatusListToken::verify(&token(key, claims), &key.public_key(), now())
}

#[test]
fn a_token_answers_for_each_status_of_the_draft() {
    let key = key();
    let token = verified(&key, &claims(&list())).expect("verifies");
    assert_eq!(token.status(&reference(0), now()), Ok(Status::Valid));
    assert_eq!(token.status(&reference(1), now()), Ok(Status::Revoked));
    assert_eq!(token.status(&reference(2), now()), Ok(Status::Suspended));
    assert_eq!(
        token.status(&reference(3), now()),
        Ok(Status::ApplicationSpecific(3))
    );
    assert_eq!(token.status(&reference(7), now()), Ok(Status::Valid));
}

/// The claims of section 5.1's example, over the list of section 4.2's: signed here
/// by a key of ours, since the draft's own signing key is not what is being tested.
#[test]
fn the_claims_of_the_drafts_example_token_are_read() {
    let key = key();
    let payload = json!({
        "sub": URI,
        "iat": ISSUED_AT,
        "exp": EXPIRES_AT,
        "ttl": 43_200,
        "status_list": { "bits": 1, "lst": "eNrbuRgAAhcBXQ" },
    });
    let token = verified(&key, &payload).expect("verifies");
    assert_eq!(token.ttl(), Some(Duration::from_secs(43_200)));
    assert_eq!(token.status(&reference(0), now()), Ok(Status::Revoked));
    assert_eq!(token.status(&reference(1), now()), Ok(Status::Valid));
    assert_eq!(token.status(&reference(3), now()), Ok(Status::Revoked));
    assert_eq!(token.status(&reference(15), now()), Ok(Status::Revoked));
}

#[test]
fn a_token_verifies_under_every_algorithm_a_key_can_have() {
    for algorithm in EVERY_ALGORITHM {
        let key = generated(algorithm);
        let token = verified(&key, &claims(&list())).expect("verifies");
        assert_eq!(
            token.status(&reference(1), now()),
            Ok(Status::Revoked),
            "{algorithm:?}"
        );
    }
}

#[test]
fn a_signature_by_another_key_is_refused() {
    let signer = key();
    let other = key();
    let token = token(&signer, &claims(&list()));
    assert_eq!(
        StatusListToken::verify(&token, &other.public_key(), now()),
        Err(TokenStatusListError::BadSignature)
    );
}

/// A forged list: a revoked entry made valid under the signature of a list that
/// had it revoked.
#[test]
fn a_payload_swapped_under_a_genuine_signature_is_refused() {
    let key = key();
    let genuine = token(&key, &claims(&list()));
    let mut forged_list = list();
    forged_list.set(1, 0).unwrap();
    let forged = token(&key, &claims(&forged_list));

    let [header, _, signature] = genuine.split('.').collect::<Vec<_>>()[..] else {
        panic!("a compact JWS");
    };
    let forged_payload = forged.split('.').nth(1).unwrap();
    assert_eq!(
        StatusListToken::verify(
            &format!("{header}.{forged_payload}.{signature}"),
            &key.public_key(),
            now()
        ),
        Err(TokenStatusListError::BadSignature)
    );
}

#[test]
fn an_unsigned_token_is_refused() {
    let key = key();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(json!({ "alg": "none", "typ": TYP }).to_string());
    let payload = b64.encode(claims(&list()).to_string());
    for token in [
        format!("{header}.{payload}."),
        format!("{header}.{payload}"),
    ] {
        assert_eq!(
            StatusListToken::verify(&token, &key.public_key(), now()),
            Err(TokenStatusListError::BadSignature)
        );
    }
}

/// Section 5.1: *the JWT type MUST be `statuslist+jwt`*. A credential's `typ`, or a
/// bare JWT, is not one, so a signed token of another purpose cannot stand as a list.
#[test]
fn a_token_of_another_type_is_refused() {
    let key = key();
    let payload = claims(&list());
    for typ in [
        json!("JWT"),
        json!("dc+sd-jwt"),
        json!("application/statuslist+jwt"),
        json!("statuslist+cwt"),
        json!(7),
    ] {
        let mut header = header(&key);
        header["typ"] = typ.clone();
        let token = forge(&key, &header, &payload);
        assert!(
            matches!(
                StatusListToken::verify(&token, &key.public_key(), now()),
                Err(TokenStatusListError::WrongTyp(_))
            ),
            "typ {typ} was accepted"
        );
    }
    let mut untyped = header(&key);
    untyped.as_object_mut().unwrap().remove("typ");
    assert_eq!(
        StatusListToken::verify(&forge(&key, &untyped, &payload), &key.public_key(), now()),
        Err(TokenStatusListError::WrongTyp(String::new()))
    );
}

#[test]
fn a_claim_that_is_missing_or_of_the_wrong_type_is_refused() {
    let key = key();
    let base = claims(&list());
    let cases = [
        ("sub", None, "sub"),
        ("sub", Some(json!(7)), "sub"),
        ("iat", None, "iat"),
        ("iat", Some(json!("1686920170")), "iat"),
        ("exp", Some(json!("2291720170")), "exp"),
        ("exp", Some(json!(null)), "exp"),
        ("status_list", None, "status_list"),
        ("status_list", Some(json!("eNrbuRgAAhcBXQ")), "status_list"),
    ];
    for (name, replacement, claim) in cases {
        let mut payload = base.clone();
        match &replacement {
            Some(value) => payload[name] = value.clone(),
            None => {
                payload.as_object_mut().unwrap().remove(name);
            }
        }
        assert_eq!(
            verified(&key, &payload).err(),
            Some(TokenStatusListError::BadClaim(claim)),
            "{name} as {replacement:?}"
        );
    }
}

#[test]
fn a_list_that_cannot_be_read_is_refused() {
    let key = key();
    let lst = list().encode();
    let cases = [
        (
            json!({ "lst": lst }),
            TokenStatusListError::BadClaim("bits"),
        ),
        (
            json!({ "bits": "2", "lst": lst }),
            TokenStatusListError::BadClaim("bits"),
        ),
        (
            json!({ "bits": -2, "lst": lst }),
            TokenStatusListError::BadClaim("bits"),
        ),
        (json!({ "bits": 2 }), TokenStatusListError::BadClaim("lst")),
        (
            json!({ "bits": 2, "lst": 7 }),
            TokenStatusListError::BadClaim("lst"),
        ),
        (
            json!({ "bits": 3, "lst": lst }),
            TokenStatusListError::UnsupportedBits(3),
        ),
        (
            json!({ "bits": 0, "lst": lst }),
            TokenStatusListError::UnsupportedBits(0),
        ),
        // Past a byte, where a cast would wrap 257 to a width that is allowed.
        (
            json!({ "bits": 257, "lst": lst }),
            TokenStatusListError::UnsupportedBits(257),
        ),
        (
            json!({ "bits": 2, "lst": "not base64!" }),
            TokenStatusListError::NotBase64Url,
        ),
    ];
    for (status_list, expected) in cases {
        let mut payload = claims(&list());
        payload["status_list"] = status_list.clone();
        assert_eq!(
            verified(&key, &payload).err(),
            Some(expected),
            "{status_list}"
        );
    }
}

#[test]
fn a_ttl_is_a_positive_number_or_it_is_refused() {
    let key = key();
    for (ttl, expected) in [
        (json!(43_200), Some(Duration::from_secs(43_200))),
        (json!(0.5), Some(Duration::from_millis(500))),
    ] {
        let mut payload = claims(&list());
        payload["ttl"] = ttl;
        assert_eq!(verified(&key, &payload).unwrap().ttl(), expected);
    }
    let mut payload = claims(&list());
    payload.as_object_mut().unwrap().remove("ttl");
    assert_eq!(verified(&key, &payload).unwrap().ttl(), None);

    for ttl in [
        json!(0),
        json!(-1),
        json!("43200"),
        json!(null),
        json!(true),
    ] {
        let mut payload = claims(&list());
        payload["ttl"] = ttl.clone();
        assert_eq!(
            verified(&key, &payload).err(),
            Some(TokenStatusListError::BadClaim("ttl")),
            "ttl {ttl}"
        );
    }
}

#[test]
fn a_token_with_no_exp_does_not_expire() {
    let key = key();
    let mut payload = claims(&list());
    payload.as_object_mut().unwrap().remove("exp");
    let token = verified(&key, &payload).expect("verifies");
    let far = chrono::Utc.timestamp_opt(4_000_000_000, 0).unwrap();
    assert_eq!(token.status(&reference(1), far), Ok(Status::Revoked));
}

/// RFC 7519 section 4.1.4: the token is not to be accepted on or after `exp`.
#[test]
fn an_expired_token_is_refused_at_the_instant_it_expires() {
    let key = key();
    let mut payload = claims(&list());
    payload["exp"] = json!(now().timestamp());
    assert_eq!(
        verified(&key, &payload).err(),
        Some(TokenStatusListError::Expired)
    );
    payload["exp"] = json!(now().timestamp() + 1);
    assert!(verified(&key, &payload).is_ok());
    // A NumericDate may be fractional.
    payload["exp"] = json!(now().timestamp() as f64 + 0.5);
    assert!(verified(&key, &payload).is_ok());
}

/// A list kept in a cache is verified once. It must not go on answering after the
/// status issuer said it stops being good.
#[test]
fn a_kept_token_stops_answering_when_it_expires() {
    let key = key();
    let mut payload = claims(&list());
    payload["exp"] = json!(now().timestamp() + 100);
    let token = verified(&key, &payload).expect("verifies");
    let before = chrono::Utc
        .timestamp_opt(now().timestamp() + 99, 0)
        .unwrap();
    let after = chrono::Utc
        .timestamp_opt(now().timestamp() + 100, 0)
        .unwrap();
    assert_eq!(token.status(&reference(1), before), Ok(Status::Revoked));
    assert_eq!(
        token.status(&reference(1), after),
        Err(TokenStatusListError::Expired)
    );
}

/// Section 8.3 step 4a: a list signed by the right key for some other URI must not
/// answer for this credential, or any credential could be pointed at a list in which
/// it reads valid.
#[test]
fn a_list_for_another_uri_does_not_answer() {
    let key = key();
    let token = verified(&key, &claims(&list())).expect("verifies");
    let mut elsewhere = reference(0);
    elsewhere.uri = "https://example.com/statuslists/2".to_owned();
    assert_eq!(
        token.status(&elsewhere, now()),
        Err(TokenStatusListError::SubjectMismatch {
            token: URI.to_owned(),
            reference: elsewhere.uri.clone(),
        })
    );
    // Exact, and not a prefix, a trailing slash or a different case.
    for uri in [
        "https://example.com/statuslists/1/",
        "https://example.com/statuslists/10",
        "HTTPS://example.com/statuslists/1",
        "https://example.com/statuslists/1?x",
    ] {
        elsewhere.uri = uri.to_owned();
        assert!(matches!(
            token.status(&elsewhere, now()),
            Err(TokenStatusListError::SubjectMismatch { .. })
        ));
    }
}

/// Section 8.3 step 6: an index outside the list rejects the credential.
#[test]
fn an_index_outside_the_list_is_refused() {
    let key = key();
    let token = verified(&key, &claims(&list())).expect("verifies");
    assert_eq!(
        token.status(&reference(8), now()),
        Err(TokenStatusListError::IndexOutOfBounds { idx: 8, entries: 8 })
    );
    assert_eq!(
        token.status(&reference(u64::MAX), now()),
        Err(TokenStatusListError::IndexOutOfBounds {
            idx: u64::MAX,
            entries: 8
        })
    );
}

#[test]
fn an_entry_nobody_has_registered_is_never_read_as_valid() {
    let key = key();
    let mut wide = TokenStatusList::new(8, 4).unwrap();
    wide.set(2, 0x10).unwrap();
    let token = verified(&key, &claims(&wide)).expect("verifies");
    assert_eq!(
        token.status(&reference(2), now()),
        Err(TokenStatusListError::UnregisteredStatus(0x10))
    );
}
