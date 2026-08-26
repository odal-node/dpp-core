//! The VC-JWT envelope: what it accepts, and everything it must refuse.
//!
//! These lean on the negative cases. A signature check that accepts a valid
//! credential proves very little on its own — the interesting property is that
//! it refuses a tampered one, an unsigned one, and one signed by somebody other
//! than the issuer it names.

use base64::Engine as _;
use serde_json::json;

use crate::credential::{
    CredentialBuilder, CredentialRole, DppCredentialSubject, VerificationResult,
    authenticate_access_credential, sign_access_credential,
};
use crate::did_builder::build_did_document;
use crate::test_support::temp_store;

const KEY_ID: &str = "issuer-key";
const BASE_URL: &str = "https://authority.example";
/// What `build_did_document` derives from `BASE_URL`.
const ISSUER_DID: &str = "did:web:authority.example";

fn credential_from(issuer: &str) -> crate::credential::DppAccessCredential {
    CredentialBuilder::new(
        issuer.to_owned(),
        DppCredentialSubject {
            id: "did:web:repairer.example".into(),
            name: "Repairs GmbH".into(),
            role: CredentialRole::AuthorisedRepairer,
            country: "DE".into(),
            product_groups: vec!["battery".into()],
            product_categories: Vec::new(),
        },
    )
    .expires_in_days(30)
    .build()
}

/// Sign a credential and return `(jws, issuer_did_document)`.
fn issued(issuer: &str) -> (String, serde_json::Value) {
    let store = temp_store("jwt", KEY_ID);
    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");
    let jws = sign_access_credential(&credential_from(issuer), &store, KEY_ID).expect("sign");
    (jws, doc)
}

// ── The one thing it should accept ───────────────────────────────────────────

#[test]
fn a_credential_signed_by_its_issuer_authenticates() {
    let (jws, doc) = issued(ISSUER_DID);

    let credential = authenticate_access_credential(&jws, &doc).expect("must authenticate");

    assert_eq!(credential.issuer, ISSUER_DID);
    assert_eq!(
        credential.credential_subject.role,
        CredentialRole::AuthorisedRepairer
    );
    assert_eq!(credential.credential_subject.id, "did:web:repairer.example");
}

/// Authentication says nothing about the validity window — that is a later step.
///
/// Worth pinning: if authentication silently also enforced expiry, a caller
/// could reasonably skip the claims check, and the two would drift.
#[test]
fn authentication_does_not_check_the_validity_window() {
    let store = temp_store("jwt-expired", KEY_ID);
    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");

    let expired = CredentialBuilder::new(
        ISSUER_DID.to_owned(),
        DppCredentialSubject {
            id: "did:web:repairer.example".into(),
            name: "Repairs GmbH".into(),
            role: CredentialRole::AuthorisedRepairer,
            country: "DE".into(),
            product_groups: vec!["battery".into()],
            product_categories: Vec::new(),
        },
    )
    .expires_at(chrono::Utc::now() - chrono::Duration::days(1))
    .build();

    let jws = sign_access_credential(&expired, &store, KEY_ID).expect("sign");

    assert!(
        authenticate_access_credential(&jws, &doc).is_ok(),
        "an expired credential is still authentic; expiry is the claims check's job"
    );
}

// ── Everything it must refuse ────────────────────────────────────────────────

/// A tampered payload must not verify — the core property of the whole scheme.
#[test]
fn a_tampered_payload_is_refused() {
    let (jws, doc) = issued(ISSUER_DID);

    // Re-encode the payload with an escalated role, leaving the signature alone.
    let mut parts: Vec<String> = jws.split('.').map(String::from).collect();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let raw = b64.decode(&parts[1]).expect("payload decodes");
    let mut payload: serde_json::Value = serde_json::from_slice(&raw).expect("payload is json");
    payload["credentialSubject"]["role"] = json!("recycler");
    parts[1] = b64.encode(serde_json::to_vec(&payload).expect("re-encode"));
    let tampered = parts.join(".");

    let err = authenticate_access_credential(&tampered, &doc).expect_err("must refuse");
    assert!(
        matches!(err, VerificationResult::InvalidSignature(_)),
        "expected InvalidSignature, got {err:?}"
    );
}

/// A credential signed by one key but presented against another issuer's
/// document must be refused, even though the signature itself is well-formed.
#[test]
fn a_credential_checked_against_another_issuers_document_is_refused() {
    let (jws, _own_doc) = issued(ISSUER_DID);

    // A different authority, with its own key and its own DID.
    let other_store = temp_store("jwt-other", KEY_ID);
    let other_doc =
        build_did_document(&other_store, "https://other.example", KEY_ID).expect("did document");

    let err = authenticate_access_credential(&jws, &other_doc).expect_err("must refuse");
    assert!(
        matches!(err, VerificationResult::InvalidSignature(_)),
        "expected InvalidSignature, got {err:?}"
    );
}

/// The binding check: a credential naming issuer A, signed by A, but verified
/// against a document that *does* contain the signing key yet belongs to B.
///
/// This is the case a signature check alone cannot catch. Without the explicit
/// `issuer` comparison the credential would authenticate, and the trust registry
/// would then be asked about the issuer the credential *claims* rather than the
/// one whose document was actually checked.
#[test]
fn a_document_for_a_different_did_is_refused_even_with_the_right_key() {
    let store = temp_store("jwt-relabel", KEY_ID);
    let mut doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");
    let jws = sign_access_credential(&credential_from(ISSUER_DID), &store, KEY_ID).expect("sign");

    // Same keys, relabelled subject — exactly what a substituted resolution
    // result would look like.
    doc["id"] = json!("did:web:impostor.example");

    let err = authenticate_access_credential(&jws, &doc).expect_err("must refuse");
    match err {
        VerificationResult::InvalidSignature(reason) => {
            assert!(
                reason.contains("impostor.example"),
                "the refusal should name the mismatched document: {reason}"
            );
        }
        other => panic!("expected InvalidSignature, got {other:?}"),
    }
}

/// `alg: none` must be refused. The algorithm is pinned by the verifier rather
/// than read from the attacker-supplied header.
#[test]
fn an_unsigned_alg_none_token_is_refused() {
    let (_jws, doc) = issued(ISSUER_DID);
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let header = b64.encode(br#"{"alg":"none","kid":"whatever"}"#);
    let payload = b64.encode(serde_json::to_vec(&credential_from(ISSUER_DID)).expect("serialise"));
    let forged = format!("{header}.{payload}.");

    assert!(
        authenticate_access_credential(&forged, &doc).is_err(),
        "alg:none must never authenticate"
    );
}

/// An issuer document that publishes no matching key refuses rather than
/// falling back to something more permissive.
#[test]
fn a_document_without_the_signing_key_is_refused() {
    let (jws, _doc) = issued(ISSUER_DID);
    let empty = json!({ "id": ISSUER_DID, "verificationMethod": [] });

    let err = authenticate_access_credential(&jws, &empty).expect_err("must refuse");
    assert!(
        matches!(err, VerificationResult::InvalidSignature(_)),
        "expected InvalidSignature, got {err:?}"
    );
}

/// Garbage in the credential position is a malformed credential, not a silent
/// success — and not a panic.
#[test]
fn a_payload_that_is_not_a_credential_is_refused() {
    let store = temp_store("jwt-garbage", KEY_ID);
    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");

    // Correctly signed, but the payload is not a credential.
    let jws =
        dpp_crypto::jws::signer::sign(&store, KEY_ID, &json!({ "hello": "world" })).expect("sign");

    let err = authenticate_access_credential(&jws, &doc).expect_err("must refuse");
    assert!(
        matches!(err, VerificationResult::MalformedCredential(_)),
        "expected MalformedCredential, got {err:?}"
    );
}

/// Structurally broken input is refused without panicking.
#[test]
fn malformed_input_is_refused_not_panicked_on() {
    let (_jws, doc) = issued(ISSUER_DID);

    for bad in ["", ".", "..", "not-a-jws", "a.b", "a.b.c.d", "!!!.???.***"] {
        assert!(
            authenticate_access_credential(bad, &doc).is_err(),
            "{bad:?} must be refused"
        );
    }
}

/// The signed bytes are JCS-canonical, so two serialisations of one credential
/// produce one signature. If this ever stops holding, an issuer and a verifier
/// could disagree about a document they both consider correct.
#[test]
fn signing_is_stable_across_serialisations_of_the_same_credential() {
    let store = temp_store("jwt-canon", KEY_ID);
    let credential = credential_from(ISSUER_DID);

    let first = sign_access_credential(&credential, &store, KEY_ID).expect("sign");
    let second = sign_access_credential(&credential, &store, KEY_ID).expect("sign again");

    assert_eq!(
        first, second,
        "the same credential must sign to the same bytes"
    );
}
