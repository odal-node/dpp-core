//! Where the passport sits in the credential: under one claim, with only the
//! claims the Issuer set at the root.

use std::collections::BTreeSet;

use base64::Engine;
use serde_json::{Map, Value, json};

use dpp_crypto::sd_jwt::SdJwt;
use dpp_domain::ProductGroup;

use super::test_fixtures::*;
use super::{Issuance, PASSPORT_CLAIM, SdJwtVcError, issue, vct_for, verify};
use crate::test_support::temp_store;

/// The claim names a reader gives a meaning to at the root of a credential: the
/// JWT registrations of RFC 7519 clause 4.1, `cnf` (RFC 7800), and the profile's
/// own (draft-ietf-oauth-sd-jwt-vc-19 clause 2.2.2).
const REGISTERED: [&str; 12] = [
    "iss",
    "sub",
    "aud",
    "exp",
    "nbf",
    "iat",
    "jti",
    "cnf",
    "vct",
    "vct#integrity",
    "aka_vcts",
    "status",
];

/// The Issuer-signed JWT's claims as signed, with no Disclosure processed.
fn signed_claims(credential: &SdJwt) -> Map<String, Value> {
    let part = credential.jwt().split('.').nth(1).expect("a compact JWS");
    serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(part)
            .expect("base64url"),
    )
    .expect("the payload is a JSON object")
}

fn root_names(claims: &Map<String, Value>) -> BTreeSet<&str> {
    claims.keys().map(String::as_str).collect()
}

fn issue_payload(store: &dpp_crypto::keystore::KeyStore, payload: &Value) -> SdJwt {
    issue(
        store,
        KEY_ID,
        payload,
        &policy(),
        &Issuance {
            issuer: ISSUER,
            vct: &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
            issued_at: ISSUED_AT,
            holder_key: None,
        },
    )
    .expect("issue")
}

/// The whole of what a reader finds at the root, and nothing a passport put there.
#[test]
fn the_root_holds_what_the_issuer_set_and_the_passport_sits_under_one_claim() {
    let store = temp_store("sdjwtvc-root", KEY_ID);
    let unbound = signed_claims(&issued(&store));
    assert_eq!(
        root_names(&unbound),
        BTreeSet::from(["iss", "vct", "iat", "_sd_alg", PASSPORT_CLAIM])
    );

    let holder = temp_store("sdjwtvc-root-holder", "holder")
        .load_key("holder")
        .expect("the key was just generated");
    let bound = issue(
        &store,
        KEY_ID,
        &payload(),
        &policy(),
        &Issuance {
            issuer: ISSUER,
            vct: &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
            issued_at: ISSUED_AT,
            holder_key: Some(holder.verifying_key.to_bytes()),
        },
    )
    .expect("issue");
    assert_eq!(
        root_names(&signed_claims(&bound)),
        BTreeSet::from(["iss", "vct", "iat", "cnf", "_sd_alg", PASSPORT_CLAIM])
    );
}

/// A passport member that shares a name with a registered claim is read as the
/// passport's, and never as the credential's. Each value here would change what
/// the credential means if it reached the root: an `exp` long past, an `nbf` not
/// yet reached, an `iss` and a `vct` that are somebody else's, an `iat` that is not
/// a date, and a `status` that is a string.
#[test]
fn a_passport_member_named_like_a_registered_claim_stays_in_the_passport() {
    let store = temp_store("sdjwtvc-collide", KEY_ID);
    let impostors = [
        ("iss", json!("https://someone.else.example")),
        ("sub", json!("a subject")),
        ("aud", json!("https://audience.example")),
        ("exp", json!(1)),
        ("nbf", json!(ISSUED_AT + 1_000_000)),
        ("iat", json!("not-a-date")),
        ("jti", json!("an identifier")),
        ("cnf", json!("not a confirmation")),
        ("vct", json!("tag:elsewhere:vct:other:1")),
        ("vct#integrity", json!("sha256-not-a-digest")),
        ("aka_vcts", json!("not a list")),
        ("status", json!("published")),
    ];
    assert_eq!(
        impostors.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        REGISTERED,
        "control: every registered name is tried"
    );

    let mut passport = payload();
    for (name, value) in &impostors {
        passport[*name] = value.clone();
    }
    let credential = issue_payload(&store, &passport);

    assert_eq!(
        root_names(&signed_claims(&credential)),
        BTreeSet::from(["iss", "vct", "iat", "_sd_alg", PASSPORT_CLAIM]),
        "a passport member reached the root of the credential"
    );

    // Read in full, and at an instant the impostor `nbf` and `exp` would refuse.
    let claims = verify(
        &credential.serialise(),
        &public_key_b64(&store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        now(),
        None,
    )
    .expect("the credential's own claims are well-formed whatever the passport holds");
    assert_eq!(claims["iss"], json!(ISSUER));
    assert_eq!(claims["iat"], json!(ISSUED_AT));
    for (name, value) in &impostors {
        assert_eq!(
            &claims[PASSPORT_CLAIM][*name], value,
            "{name} did not come back as the passport's"
        );
    }
}

/// The case that was real: a passport's lifecycle `status` is a string, and at the
/// root `status` is the reference to a Token Status List.
#[test]
fn the_passports_own_status_is_not_the_credentials() {
    let store = temp_store("sdjwtvc-status", KEY_ID);
    let mut passport = payload();
    passport["status"] = json!("draft");
    let credential = issue_payload(&store, &passport);

    assert!(
        signed_claims(&credential).get("status").is_none(),
        "the passport's status is the credential's"
    );

    let claims = verify(
        &credential.serialise(),
        &public_key_b64(&store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        now(),
        None,
    )
    .expect("verifies");
    assert!(claims.get("status").is_none());
    assert_eq!(claims[PASSPORT_CLAIM]["status"], json!("draft"));
}

/// Nothing here follows a `status` reference, so the shape is all that can be
/// held to. A string where the profile expects a status mechanism is refused rather
/// than passed for one.
#[test]
fn a_status_that_is_not_an_object_is_refused() {
    let store = temp_store("sdjwtvc-status-shape", KEY_ID);

    let object = json!({ "status_list": { "idx": 0, "uri": "https://status.example/1" } });
    assert!(
        check(
            &store,
            &credential_with(&store, &[("status", object)]),
            now()
        )
        .is_ok(),
        "control: an object is accepted, and not followed"
    );

    for value in [json!("draft"), json!(42), json!(["a"]), json!(null)] {
        assert_eq!(
            check(
                &store,
                &credential_with(&store, &[("status", value.clone())]),
                now()
            ),
            Err(SdJwtVcError::MalformedStatus),
            "status as {value} was accepted"
        );
    }
}
