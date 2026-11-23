//! A credential that can be withdrawn: the reference [`issue`] writes, what
//! [`verify`] holds it to, and the path from the one to a status.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::SdJwt;
use dpp_domain::ProductGroup;

use super::test_fixtures::*;
use super::{Issuance, PASSPORT_CLAIM, SdJwtVcError, issue, vct_for, verify};
use crate::common::jwt::jwt_part;
use crate::test_support::temp_store;
use crate::token_status_list::{
    Status, StatusListToken, StatusReference, TYP, TokenStatusList, TokenStatusListError,
};

const LIST_URI: &str = "https://passports.operator.example/status/1";

fn reference(idx: u64) -> StatusReference {
    StatusReference {
        idx,
        uri: LIST_URI.to_owned(),
    }
}

fn issued_with(store: &KeyStore, status: Option<&StatusReference>, passport: &Value) -> SdJwt {
    issue(
        store,
        KEY_ID,
        passport,
        &policy(),
        &Issuance {
            issuer: ISSUER,
            vct: &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
            issued_at: ISSUED_AT,
            holder_key: None,
            status,
        },
    )
    .expect("issue")
}

fn verified(store: &KeyStore, credential: &SdJwt) -> serde_json::Map<String, Value> {
    verify(
        &credential.serialise(),
        &public_key(store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        now(),
        None,
    )
    .expect("verifies")
}

/// A Status List Token over `list`, signed by the issuer's key as a status issuer
/// who is also the credential's issuer would.
fn status_list_token(
    store: &KeyStore,
    uri: &str,
    list: &TokenStatusList,
    at: DateTime<Utc>,
) -> String {
    let claims = json!({
        "sub": uri,
        "iat": at.timestamp(),
        "ttl": 3600,
        "status_list": { "bits": list.bits(), "lst": list.encode() },
    });
    let kid = store
        .public_key(KEY_ID)
        .expect("the fixture key")
        .thumbprint_uri()
        .expect("a valid public key");
    dpp_crypto::jws::sign_typed(store, KEY_ID, &claims, &kid, Some(TYP), None)
        .expect("the fixture key signs")
}

#[test]
fn an_issued_credential_carries_the_reference_in_the_clear() {
    let store = temp_store("sdjwtvc-status-issue", KEY_ID);
    let credential = issued_with(&store, Some(&reference(42)), &payload());

    let signed = jwt_part(credential.jwt(), 1).expect("a JSON payload");
    assert_eq!(
        signed["status"],
        json!({ "status_list": { "idx": 42, "uri": LIST_URI } }),
        "the draft's shape, in the signed payload and not in a Disclosure"
    );

    let claims = verified(&store, &credential);
    assert_eq!(
        StatusReference::from_claims(&claims),
        Ok(Some(reference(42)))
    );
}

#[test]
fn no_reference_means_no_status_claim() {
    let store = temp_store("sdjwtvc-status-none", KEY_ID);
    let credential = issued_with(&store, None, &payload());
    let signed = jwt_part(credential.jwt(), 1).expect("a JSON payload");
    assert!(signed.get("status").is_none());
    assert_eq!(
        StatusReference::from_claims(&verified(&store, &credential)),
        Ok(None)
    );
}

/// A passport has a lifecycle `status` of its own, a string. The credential's is the
/// reference. Both exist and neither is read as the other.
#[test]
fn the_reference_and_the_passports_own_status_stay_apart() {
    let store = temp_store("sdjwtvc-status-apart", KEY_ID);
    let mut passport = payload();
    passport["status"] = json!("published");
    let credential = issued_with(&store, Some(&reference(1)), &passport);
    let claims = verified(&store, &credential);

    assert_eq!(claims["status"], reference(1).to_claim());
    assert_eq!(claims[PASSPORT_CLAIM]["status"], json!("published"));
    assert_eq!(
        StatusReference::from_claims(&claims),
        Ok(Some(reference(1)))
    );
}

/// What the issue asks for, from end to end: a credential issued once, and a status
/// that moves while the credential does not.
#[test]
fn a_credential_is_withdrawn_by_the_list_and_not_by_the_credential() {
    let store = temp_store("sdjwtvc-status-withdraw", KEY_ID);
    let credential = issued_with(&store, Some(&reference(5)), &payload());
    let key = public_key(&store);
    let reference_of = |claims: &serde_json::Map<String, Value>| {
        StatusReference::from_claims(claims)
            .expect("a readable reference")
            .expect("the credential carries one")
    };

    let mut list = TokenStatusList::new(2, 16).unwrap();
    let published = status_list_token(&store, LIST_URI, &list, now());
    let token = StatusListToken::verify(&published, &key, now()).expect("a genuine list");
    let claims = verified(&store, &credential);
    assert_eq!(
        token.status(&reference_of(&claims), now()),
        Ok(Status::Valid)
    );

    list.set(5, 2).unwrap();
    let suspended = status_list_token(&store, LIST_URI, &list, now());
    let token = StatusListToken::verify(&suspended, &key, now()).expect("a genuine list");
    assert_eq!(
        token.status(&reference_of(&claims), now()),
        Ok(Status::Suspended)
    );

    list.set(5, 1).unwrap();
    let revoked = status_list_token(&store, LIST_URI, &list, now());
    let token = StatusListToken::verify(&revoked, &key, now()).expect("a genuine list");
    assert_eq!(
        token.status(&reference_of(&claims), now()),
        Ok(Status::Revoked)
    );

    // The credential was never touched, and it verifies as it did.
    assert_eq!(verified(&store, &credential), claims);
}

/// A list published for another URI is not this credential's, even under the same
/// key.
#[test]
fn a_list_for_another_credentials_uri_does_not_answer() {
    let store = temp_store("sdjwtvc-status-uri", KEY_ID);
    let credential = issued_with(&store, Some(&reference(0)), &payload());
    let claims = verified(&store, &credential);
    let wanted = StatusReference::from_claims(&claims).unwrap().unwrap();

    let other = "https://passports.operator.example/status/2";
    let list = TokenStatusList::new(1, 8).unwrap();
    let token = status_list_token(&store, other, &list, now());
    let token = StatusListToken::verify(&token, &public_key(&store), now()).unwrap();
    assert_eq!(
        token.status(&wanted, now()),
        Err(TokenStatusListError::SubjectMismatch {
            token: other.to_owned(),
            reference: LIST_URI.to_owned(),
        })
    );
}

#[test]
fn a_reference_that_is_not_one_is_refused_on_verification() {
    let store = temp_store("sdjwtvc-status-refused", KEY_ID);
    let uri = "https://status.example/1";
    let refused = [
        json!({ "status_list": "x" }),
        json!({ "status_list": { "uri": uri } }),
        json!({ "status_list": { "idx": -1, "uri": uri } }),
        json!({ "status_list": { "idx": "0", "uri": uri } }),
        json!({ "status_list": { "idx": 0 } }),
        json!({ "status_list": { "idx": 0, "uri": 7 } }),
    ];
    for status in refused {
        assert_eq!(
            check(
                &store,
                &credential_with(&store, &[("status", status.clone())]),
                now()
            ),
            Err(SdJwtVcError::MalformedStatus),
            "status {status} was accepted"
        );
    }
}

/// The draft lets a credential carry several status mechanisms. One this crate does
/// not read is passed, and a credential that names a list beside it is still read.
#[test]
fn a_status_with_a_mechanism_this_crate_does_not_read_passes() {
    let store = temp_store("sdjwtvc-status-other", KEY_ID);
    let other = json!({ "other_mechanism": { "id": 7 } });
    assert!(
        check(
            &store,
            &credential_with(&store, &[("status", other)]),
            now()
        )
        .is_ok()
    );

    let both = json!({
        "other_mechanism": { "id": 7 },
        "status_list": { "idx": 3, "uri": "https://status.example/1" },
    });
    assert!(check(&store, &credential_with(&store, &[("status", both)]), now()).is_ok());
}
