//! The reason a VC-JWT is refused, as a type.
//!
//! [`AuthenticationFailure`] names each thing an issuer would go and fix, and the
//! untyped function is the typed one with the reason folded away, so the two
//! cannot disagree about a token. The envelope's own acceptance and refusal cases
//! are in `jwt_tests`, whose fixtures these share.

use base64::Engine as _;
use serde_json::json;

use super::jwt_tests::{BASE_URL, ISSUER_DID, KEY_ID, credential_from, signed};
use crate::credential::{
    AuthenticationFailure, VerificationResult, authenticate_access_credential,
    authenticate_access_credential_with_reason,
};
use crate::did_builder::build_did_document;
use crate::test_support::temp_store;
use dpp_crypto::keystore::KeyStore;

// ── The reason, as a type ────────────────────────────────────────────────────

/// A fixture with a document, a key the document publishes, and a way to sign a
/// credential under it with any header: what every refusal below is built from.
struct Signer {
    store: KeyStore,
    doc: serde_json::Value,
    kid: String,
}

fn signer(name: &str) -> Signer {
    let store = temp_store(name, KEY_ID);
    let doc = build_did_document(&store, BASE_URL, KEY_ID).expect("did document");
    let kid = doc["verificationMethod"][0]["id"]
        .as_str()
        .expect("the document publishes a key")
        .to_owned();
    Signer { store, doc, kid }
}

/// Every way a token can be refused comes back as its own variant, so a caller can
/// say which thing to fix without matching on text. Each case differs from the
/// one that authenticates in exactly one respect.
#[test]
fn each_refusal_is_its_own_reason() {
    let s = signer("jwt-reasons");
    let credential = serde_json::to_value(credential_from(ISSUER_DID)).expect("serialise");
    let good = signed(&s.store, &credential, &s.kid, Some("vc+jwt"), Some("vc"));
    assert!(authenticate_access_credential_with_reason(&good, &s.doc).is_ok());

    // The signature does not verify: a key matched, the bytes were changed.
    let mut parts: Vec<String> = good.split('.').map(String::from).collect();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mut payload: serde_json::Value =
        serde_json::from_slice(&b64.decode(&parts[1]).expect("payload decodes")).expect("json");
    payload["credentialSubject"]["role"] = json!("recycler");
    parts[1] = b64.encode(serde_json::to_vec(&payload).expect("re-encode"));
    assert_eq!(
        authenticate_access_credential_with_reason(&parts.join("."), &s.doc),
        Err(AuthenticationFailure::SignatureDoesNotVerify)
    );

    // The document publishes no key the token's `kid` names.
    let empty = json!({ "id": ISSUER_DID, "verificationMethod": [] });
    assert_eq!(
        authenticate_access_credential_with_reason(&good, &empty),
        Err(AuthenticationFailure::NoMatchingKey {
            document: Some(ISSUER_DID.to_owned()),
            kid: Some(s.kid.clone()),
        })
    );

    // Authentic, and not typed as a credential.
    let untyped = signed(&s.store, &credential, &s.kid, None, None);
    assert!(matches!(
        authenticate_access_credential_with_reason(&untyped, &s.doc),
        Err(AuthenticationFailure::NotTypedAsCredential { reason }) if reason.contains("typ")
    ));

    // Authentic, typed, and the payload is not a credential.
    let not_a_credential = signed(
        &s.store,
        &json!({ "hello": "world" }),
        &s.kid,
        Some("vc+jwt"),
        Some("vc"),
    );
    assert!(matches!(
        authenticate_access_credential_with_reason(&not_a_credential, &s.doc),
        Err(AuthenticationFailure::MalformedPayload { .. })
    ));

    // Authentic, typed, a credential, and it names another issuer than the
    // document it is checked against.
    let impostor_store = temp_store("jwt-reasons-impostor", KEY_ID);
    let impostor_doc = build_did_document(&impostor_store, "https://impostor.example", KEY_ID)
        .expect("did document");
    let impostor_kid = impostor_doc["verificationMethod"][0]["id"]
        .as_str()
        .expect("the document publishes a key");
    let claims_a = signed(
        &impostor_store,
        &credential,
        impostor_kid,
        Some("vc+jwt"),
        Some("vc"),
    );
    assert_eq!(
        authenticate_access_credential_with_reason(&claims_a, &impostor_doc),
        Err(AuthenticationFailure::IssuerMismatch {
            claimed: ISSUER_DID.to_owned(),
            document: "did:web:impostor.example".to_owned(),
        })
    );
}

/// Every token signed before keys were named by thumbprint carries a bare
/// fingerprint as its `kid`. No document names a key that way, so the reason is
/// the key, and says so; it is not a bad signature.
#[test]
fn a_kid_that_is_a_bare_fingerprint_names_no_key() {
    let s = signer("jwt-bare-kid");
    let credential = serde_json::to_value(credential_from(ISSUER_DID)).expect("serialise");
    let fingerprint = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
    let jws = signed(
        &s.store,
        &credential,
        fingerprint,
        Some("vc+jwt"),
        Some("vc"),
    );

    assert_eq!(
        authenticate_access_credential_with_reason(&jws, &s.doc),
        Err(AuthenticationFailure::NoMatchingKey {
            document: Some(ISSUER_DID.to_owned()),
            kid: Some(fingerprint.to_owned()),
        })
    );
}

/// The untyped function answers every token as it did before the reason had a
/// type: the same [`VerificationResult`] variant and the same prose, so a caller
/// that already shows it sees no change.
///
/// Each expected answer is written out rather than derived from the typed
/// function. The untyped one is defined as the typed one folded, so comparing
/// the two would pass whatever the folding did; only a pinned answer catches a
/// refusal moved to another variant or reworded.
#[test]
fn the_untyped_function_answers_as_it_always_has() {
    let s = signer("jwt-agree");
    let credential = serde_json::to_value(credential_from(ISSUER_DID)).expect("serialise");
    let good = signed(&s.store, &credential, &s.kid, Some("vc+jwt"), Some("vc"));
    let empty = json!({ "id": ISSUER_DID, "verificationMethod": [] });
    let no_id = json!({ "verificationMethod": [] });
    let other_store = temp_store("jwt-agree-other", KEY_ID);
    let other_doc =
        build_did_document(&other_store, "https://other.example", KEY_ID).expect("did document");

    // A key matches and the bytes it signed were changed.
    let mut tampered: Vec<String> = good.split('.').map(String::from).collect();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let mut payload: serde_json::Value =
        serde_json::from_slice(&b64.decode(&tampered[1]).expect("payload decodes")).expect("json");
    payload["credentialSubject"]["role"] = json!("recycler");
    tampered[1] = b64.encode(serde_json::to_vec(&payload).expect("re-encode"));
    let tampered = tampered.join(".");

    let no_key = |document: &str, kid: &str| {
        Err(VerificationResult::InvalidSignature(format!(
            "the DID document for {document} publishes no key matching kid {kid}"
        )))
    };
    let cases = [
        ("authenticates", &good, &s.doc, Ok(ISSUER_DID.to_owned())),
        ("no key", &good, &empty, no_key(ISSUER_DID, &s.kid)),
        (
            "another DID's document",
            &good,
            &other_doc,
            no_key("did:web:other.example", &s.kid),
        ),
        (
            "a document with no id",
            &good,
            &no_id,
            no_key("(no id)", &s.kid),
        ),
        (
            "a signature that does not verify",
            &tampered,
            &s.doc,
            Err(VerificationResult::InvalidSignature(
                "signature did not verify against the issuer's published key".to_owned(),
            )),
        ),
        (
            "not typed as a credential",
            &signed(&s.store, &credential, &s.kid, None, None),
            &s.doc,
            Err(VerificationResult::InvalidSignature(
                "the token's typ is absent, not vc+jwt".to_owned(),
            )),
        ),
        (
            "not a JWS",
            &"not-a-jws".to_owned(),
            &s.doc,
            no_key(ISSUER_DID, "(none)"),
        ),
        (
            "an unreadable header",
            &"a.b.c".to_owned(),
            &s.doc,
            no_key(ISSUER_DID, "(none)"),
        ),
    ];
    for (case, jws, doc, expected) in cases {
        assert_eq!(
            authenticate_access_credential(jws, doc).map(|c| c.issuer),
            expected,
            "{case}"
        );
    }

    // The payload's own error text is serde's, so only its variant and the
    // sentence it opens with are this crate's to pin.
    let not_a_credential = signed(
        &s.store,
        &json!({ "hello": "world" }),
        &s.kid,
        Some("vc+jwt"),
        Some("vc"),
    );
    assert!(matches!(
        authenticate_access_credential(&not_a_credential, &s.doc),
        Err(VerificationResult::MalformedCredential(reason))
            if reason.starts_with("payload is not a credential: ")
    ));
}

/// A token with no `kid`, checked against a document with no `id`, is refused
/// with neither: the typed reason says each is absent rather than carrying the
/// placeholder the prose prints for it.
#[test]
fn an_absent_kid_and_document_id_are_absent() {
    let failure = authenticate_access_credential_with_reason(
        "not-a-jws",
        &json!({ "verificationMethod": [] }),
    )
    .expect_err("there is no key to find");

    assert_eq!(
        failure,
        AuthenticationFailure::NoMatchingKey {
            document: None,
            kid: None,
        }
    );
    assert_eq!(
        failure.to_string(),
        "the DID document for (no id) publishes no key matching kid (none)"
    );
}
