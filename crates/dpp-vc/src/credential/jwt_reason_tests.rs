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
    AuthenticationFailure, authenticate_access_credential,
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
            document: ISSUER_DID.to_owned(),
            kid: s.kid.clone(),
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
            document: ISSUER_DID.to_owned(),
            kid: fingerprint.to_owned(),
        })
    );
}

/// The two functions are one check. The untyped one is the typed one with the
/// reason folded into a [`VerificationResult`], so a token cannot pass one and be
/// refused by the other, and the prose a caller already shows is unchanged.
#[test]
fn the_untyped_function_is_the_typed_one_folded() {
    let s = signer("jwt-agree");
    let credential = serde_json::to_value(credential_from(ISSUER_DID)).expect("serialise");
    let empty = json!({ "id": ISSUER_DID, "verificationMethod": [] });
    let other_store = temp_store("jwt-agree-other", KEY_ID);
    let other_doc =
        build_did_document(&other_store, "https://other.example", KEY_ID).expect("did document");

    let tokens = [
        signed(&s.store, &credential, &s.kid, Some("vc+jwt"), Some("vc")),
        signed(&s.store, &credential, &s.kid, None, None),
        signed(
            &s.store,
            &json!({ "hello": "world" }),
            &s.kid,
            Some("vc+jwt"),
            Some("vc"),
        ),
        "not-a-jws".to_owned(),
        "a.b.c".to_owned(),
    ];
    for jws in &tokens {
        for doc in [&s.doc, &empty, &other_doc] {
            let untyped = authenticate_access_credential(jws, doc).map(|c| c.issuer);
            let typed = authenticate_access_credential_with_reason(jws, doc)
                .map(|c| c.issuer)
                .map_err(AuthenticationFailure::into_verification_result);
            assert_eq!(untyped, typed, "{jws:?} against {doc}");
        }
    }

    // …and the prose is what it always was.
    let reason =
        authenticate_access_credential_with_reason(&tokens[0], &empty).expect_err("no key to find");
    assert_eq!(
        reason.to_string(),
        format!(
            "the DID document for {ISSUER_DID} publishes no key matching kid {}",
            s.kid
        )
    );
}
