//! The expiry [`issue`] writes: that the credential carries what it was given, that a
//! Holder cannot leave it out of a presentation or change it, and that [`verify`]
//! enforces it on what this crate issues.
//!
//! `validity.rs` holds the verifier to `exp` and `nbf` on tokens a foreign issuer
//! might emit. These are the other half: the credential this crate's issuer makes.

use base64::Engine;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

use dpp_crypto::keystore::KeyStore;
use dpp_crypto::sd_jwt::SdJwt;
use dpp_domain::ProductGroup;

use super::super::{Issuance, SdJwtVcError, issue, vct_for, verify};
use super::common::*;
use crate::common::jwt::jwt_part;
use crate::test_support::temp_store;
use crate::token_status_list::StatusReference;

fn at(seconds: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0).unwrap()
}

/// A credential issued from the fixture passport, valid from `not_before` and until
/// `expires_at`, and withdrawable through `status`.
fn issued_with(
    store: &KeyStore,
    expires_at: Option<i64>,
    not_before: Option<i64>,
    status: Option<&StatusReference>,
) -> Result<SdJwt, SdJwtVcError> {
    issue(
        store,
        KEY_ID,
        &payload(),
        &policy(),
        &Issuance {
            issuer: ISSUER,
            vct: &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
            issued_at: ISSUED_AT,
            expires_at,
            not_before,
            holder_key: None,
            status,
        },
    )
}

/// [`issued_with`] an `exp` and no status reference.
fn issued_until(
    store: &KeyStore,
    expires_at: i64,
    not_before: Option<i64>,
) -> Result<SdJwt, SdJwtVcError> {
    issued_with(store, Some(expires_at), not_before, None)
}

fn verified(
    store: &KeyStore,
    credential: &SdJwt,
    instant: DateTime<Utc>,
) -> Result<serde_json::Map<String, Value>, SdJwtVcError> {
    verify(
        &credential.serialise(),
        &public_key(store),
        &vct_for(ProductGroup::Battery, SCHEMA_VERSION),
        instant,
        None,
    )
}

#[test]
fn an_issued_credential_carries_the_exp_it_was_given() {
    let store = temp_store("sdjwtvc-exp-given", KEY_ID);
    let credential = issued_until(&store, ISSUED_AT + 3600, None).expect("issue");

    let signed = jwt_part(credential.jwt(), 1).expect("a JSON payload");
    assert_eq!(signed["exp"], json!(ISSUED_AT + 3600), "as signed");
    assert!(signed.get("nbf").is_none(), "no start was given");

    let claims = verified(&store, &credential, now()).expect("verifies");
    assert_eq!(claims["exp"], json!(ISSUED_AT + 3600), "as verified");
}

#[test]
fn a_start_is_written_only_when_one_is_given() {
    let store = temp_store("sdjwtvc-nbf-given", KEY_ID);
    let credential = issued_until(&store, ISSUED_AT + 3600, Some(ISSUED_AT + 600)).expect("issue");

    let signed = jwt_part(credential.jwt(), 1).expect("a JSON payload");
    assert_eq!(signed["nbf"], json!(ISSUED_AT + 600));
    assert_eq!(signed["exp"], json!(ISSUED_AT + 3600));
}

/// What the issue asks for: the Holder is the party who chooses what to reveal, and
/// `exp` is not among the things it can choose. Every presentation there is of this
/// credential carries the issuer's `exp`, and a Holder who edits it breaks the
/// signature.
#[test]
fn a_holder_cannot_leave_exp_out_of_a_presentation() {
    let store = temp_store("sdjwtvc-exp-holder", KEY_ID);
    let credential = issued_until(&store, EXPIRES_AT, None).expect("issue");
    assert!(
        credential.disclosures().len() > 1,
        "control: the fixture conceals several claims, so there is something to choose"
    );

    let all: Vec<String> = credential
        .disclosures()
        .iter()
        .map(|disclosure| disclosure.digest())
        .collect();
    let mut choices: Vec<Vec<String>> = vec![Vec::new(), all.clone()];
    choices.extend(all.iter().map(|digest| vec![digest.clone()]));

    for choice in &choices {
        let digests: Vec<&str> = choice.iter().map(String::as_str).collect();
        let presentation = credential.present(&digests).expect("a valid choice");
        let claims = verified(&store, &presentation, now()).expect("verifies");
        assert_eq!(
            claims["exp"],
            json!(EXPIRES_AT),
            "a presentation of {} disclosures lost the exp",
            choice.len()
        );
    }
}

#[test]
fn a_holder_who_edits_the_signed_claims_breaks_the_signature() {
    let store = temp_store("sdjwtvc-exp-edit", KEY_ID);
    let credential = issued_until(&store, ISSUED_AT + 3600, None).expect("issue");
    let [header, _, signature] = credential.jwt().split('.').collect::<Vec<_>>()[..] else {
        panic!("a compact JWS");
    };
    let signed = jwt_part(credential.jwt(), 1).expect("a JSON payload");

    let mut dropped = signed.clone();
    dropped.remove("exp");
    let mut extended = signed.clone();
    extended.insert("exp".to_owned(), json!(ISSUED_AT + 10 * 31_536_000));

    for (what, claims) in [("dropped", dropped), ("extended", extended)] {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&Value::Object(claims)).expect("serialises"));
        let forged = SdJwt::new(
            format!("{header}.{payload}.{signature}"),
            credential.disclosures().to_vec(),
        );
        assert_eq!(
            verified(&store, &forged, now()).err(),
            Some(SdJwtVcError::BadSignature),
            "an exp {what} by the Holder was accepted"
        );
    }
}

/// `verify` on what this crate issues, and not on a token built by hand: a second
/// before `exp` it verifies, at `exp` and after it, it does not, and a presentation
/// that reveals nothing is no way round it.
#[test]
fn an_issued_credential_is_refused_from_its_exp_on() {
    let store = temp_store("sdjwtvc-exp-enforced", KEY_ID);
    let expiry = ISSUED_AT + 3600;
    let credential = issued_until(&store, expiry, None).expect("issue");
    let bare = credential.present(&[]).expect("nothing chosen is a choice");

    for presented in [&credential, &bare] {
        assert!(verified(&store, presented, at(expiry - 1)).is_ok());
        for instant in [expiry, expiry + 1, expiry + 86_400] {
            assert_eq!(
                verified(&store, presented, at(instant)).err(),
                Some(SdJwtVcError::Expired { at: at(expiry) }),
                "accepted at {instant}"
            );
        }
    }
}

#[test]
fn an_issued_credential_is_refused_before_its_nbf() {
    let store = temp_store("sdjwtvc-nbf-enforced", KEY_ID);
    let start = ISSUED_AT + 600;
    let credential = issued_until(&store, ISSUED_AT + 3600, Some(start)).expect("issue");

    assert_eq!(
        verified(&store, &credential, at(start - 1)).err(),
        Some(SdJwtVcError::NotYetValid { from: at(start) })
    );
    assert!(verified(&store, &credential, at(start)).is_ok());
}

/// A credential that expires as it is issued, or starts after it ends, verifies
/// nowhere. It is refused at issue, where the Issuer can see the mistake, and not
/// found by the first Holder who presents it.
#[test]
fn a_validity_window_that_is_empty_is_refused() {
    let store = temp_store("sdjwtvc-window", KEY_ID);
    let empty = [
        ("exp at iat", ISSUED_AT, None),
        ("exp before iat", ISSUED_AT - 1, None),
        ("nbf at exp", ISSUED_AT + 3600, Some(ISSUED_AT + 3600)),
        ("nbf after exp", ISSUED_AT + 3600, Some(ISSUED_AT + 7200)),
    ];
    for (what, expires_at, not_before) in empty {
        assert_eq!(
            issued_until(&store, expires_at, not_before).err(),
            Some(SdJwtVcError::EmptyValidityWindow),
            "{what} was issued"
        );
    }

    // The nearest windows that are not empty.
    for (what, expires_at, not_before) in [
        ("exp one second after iat", ISSUED_AT + 1, None),
        (
            "nbf one second before exp",
            ISSUED_AT + 3600,
            Some(ISSUED_AT + 3599),
        ),
        ("nbf before iat", ISSUED_AT + 3600, Some(ISSUED_AT - 60)),
    ] {
        assert!(
            issued_until(&store, expires_at, not_before).is_ok(),
            "{what} was refused"
        );
    }
}

/// The floor: a credential has to be able to end. With no `exp` the only other way is
/// a status reference, and with neither nothing short of revoking the Issuer's key
/// could withdraw it, which is the gap this closes. A start does not end a credential.
#[test]
fn a_credential_with_neither_an_exp_nor_a_status_reference_is_refused() {
    let store = temp_store("sdjwtvc-unbounded-refused", KEY_ID);

    for (what, not_before) in [("nothing", None), ("only a start", Some(ISSUED_AT + 600))] {
        assert_eq!(
            issued_with(&store, None, not_before, None).err(),
            Some(SdJwtVcError::Unbounded),
            "a credential with {what} was issued"
        );
    }
}

/// With a status reference, `exp` may be left out: the list ends the credential, and
/// no `exp` is written, so a verifier that finds none is reading what the Issuer chose.
#[test]
fn a_status_reference_can_end_a_credential_that_has_no_exp() {
    let store = temp_store("sdjwtvc-status-only", KEY_ID);
    let reference = StatusReference {
        idx: 3,
        uri: "https://status.example/1".to_owned(),
    };

    for not_before in [None, Some(ISSUED_AT + 600)] {
        let credential =
            issued_with(&store, None, not_before, Some(&reference)).expect("a list can end it");
        let signed = jwt_part(credential.jwt(), 1).expect("a JSON payload");
        assert!(signed.get("exp").is_none(), "an exp was written");
        assert_eq!(signed["status"], reference.to_claim());

        let claims = verified(&store, &credential, at(4_000_000_000)).expect("verifies");
        assert_eq!(
            StatusReference::from_claims(&claims),
            Ok(Some(reference.clone()))
        );
    }
}

/// A credential with both ends at the earlier of the two, and carries both claims.
#[test]
fn an_exp_and_a_status_reference_together_write_both() {
    let store = temp_store("sdjwtvc-exp-and-status", KEY_ID);
    let reference = StatusReference {
        idx: 3,
        uri: "https://status.example/1".to_owned(),
    };
    let credential =
        issued_with(&store, Some(ISSUED_AT + 3600), None, Some(&reference)).expect("issue");

    let signed = jwt_part(credential.jwt(), 1).expect("a JSON payload");
    assert_eq!(signed["exp"], json!(ISSUED_AT + 3600));
    assert_eq!(signed["status"], reference.to_claim());
}
