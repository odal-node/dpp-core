//! `resolve_verification_key`: which key a JWS names, and everything that must
//! make it name none.
//!
//! The resolver reads the header and the document and never checks a signature,
//! so the tokens here carry a placeholder one. Each rejection is built so that
//! only the rule under test can be the cause: the rest of the document is valid,
//! and a control shows that it resolves.

use base64::Engine;
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

use super::algorithm::KeyAlgorithm;
use super::verifier::resolve_verification_key;

const DID: &str = "did:web:issuer.example";

fn public_key() -> Vec<u8> {
    SigningKey::generate(&mut crate::os_rng())
        .verifying_key()
        .as_bytes()
        .to_vec()
}

fn x(public_key: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key)
}

/// The DID URL of the verification method this crate's builder would give a key.
fn vm_id(public_key: &[u8]) -> String {
    format!("{DID}#{}", KeyAlgorithm::Ed25519.thumbprint_uri(public_key))
}

/// A verification method for `public_key`, identified as the builder would.
fn vm(public_key: &[u8]) -> Value {
    json!({
        "id": vm_id(public_key),
        "type": "JsonWebKey",
        "controller": DID,
        "publicKeyJwk": KeyAlgorithm::Ed25519.published_jwk(public_key),
    })
}

fn document(methods: &[Value]) -> Value {
    let ids: Vec<&Value> = methods.iter().map(|m| &m["id"]).collect();
    json!({"id": DID, "verificationMethod": methods, "assertionMethod": ids})
}

/// A token naming `kid` under `alg`. Only the header is read.
fn token(header: &Value) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!("{}.e30.c2ln", b64.encode(header.to_string()))
}

fn naming(kid: &str) -> String {
    token(&json!({"alg": "EdDSA", "kid": kid}))
}

/// The control. A rejection below means something only if this resolves.
#[test]
fn a_kid_that_is_a_verification_method_id_resolves_to_its_key() {
    let key = public_key();
    let doc = document(&[vm(&key)]);

    assert_eq!(
        resolve_verification_key(&doc, &naming(&vm_id(&key))),
        Some(x(&key))
    );
}

/// An identifier is the key's own, so a key resolves from wherever it sits in the
/// document. A position in a list is not an identifier: it changes when a revoked
/// key drops out, and a token's `kid` is fixed when it is issued.
#[test]
fn each_key_resolves_by_its_own_identifier_in_any_order() {
    let (older, newer) = (public_key(), public_key());

    for doc in [
        document(&[vm(&newer), vm(&older)]),
        document(&[vm(&older), vm(&newer)]),
    ] {
        assert_eq!(
            resolve_verification_key(&doc, &naming(&vm_id(&older))),
            Some(x(&older))
        );
        assert_eq!(
            resolve_verification_key(&doc, &naming(&vm_id(&newer))),
            Some(x(&newer))
        );
    }
}

#[test]
fn a_kid_that_names_no_verification_method_resolves_to_nothing() {
    let key = public_key();
    let doc = document(&[vm(&key)]);
    let thumbprint = KeyAlgorithm::Ed25519.thumbprint_uri(&key);

    for kid in [
        // Another key's identifier.
        vm_id(&public_key()),
        // The old fingerprint form, and a position.
        hex::encode(&key),
        format!("{DID}#key-1"),
        // The thumbprint without the DID URL it must be part of.
        thumbprint.clone(),
        // Another DID's method, with the right fragment.
        format!("did:web:other.example#{thumbprint}"),
        String::new(),
    ] {
        assert_eq!(
            resolve_verification_key(&doc, &naming(&kid)),
            None,
            "kid: {kid:?}"
        );
    }
}

/// A token that does not say which key signed it is not verified against one
/// chosen for it, even when the document has only one.
#[test]
fn a_token_without_a_kid_resolves_to_nothing() {
    let key = public_key();
    let doc = document(&[vm(&key)]);

    assert_eq!(
        resolve_verification_key(&doc, &token(&json!({"alg": "EdDSA"}))),
        None
    );
    assert_eq!(
        resolve_verification_key(&doc, &token(&json!({"alg": "EdDSA", "kid": 7}))),
        None
    );
    assert_eq!(resolve_verification_key(&doc, "not-a-jws"), None);
}

/// The verification relationship is checked: a method that is not in
/// `assertionMethod` is not a signer.
#[test]
fn a_method_not_listed_in_assertion_method_is_not_a_signer() {
    let key = public_key();
    let id = vm_id(&key);

    let authentication_only = json!({
        "id": DID,
        "verificationMethod": [vm(&key)],
        "authentication": [id],
        "assertionMethod": [],
    });
    let no_relationship = json!({"id": DID, "verificationMethod": [vm(&key)]});

    for doc in [authentication_only, no_relationship] {
        assert_eq!(resolve_verification_key(&doc, &naming(&id)), None);
    }
}

/// Only `kty: OKP, crv: Ed25519` keys resolve. The identifier is built from the
/// same `x`, so it is the JWK's type and not the identifier that refuses.
#[test]
fn only_an_ed25519_jwk_resolves() {
    let key = public_key();
    let id = vm_id(&key);
    let with = |jwk: Value| {
        document(&[json!({"id": id, "type": "JsonWebKey", "controller": DID, "publicKeyJwk": jwk})])
    };

    for jwk in [
        json!({"kty": "OKP", "crv": "X25519", "x": x(&key)}),
        json!({"kty": "EC", "crv": "P-256", "x": x(&key)}),
        json!({"x": x(&key)}),
    ] {
        assert_eq!(
            resolve_verification_key(&with(jwk.clone()), &naming(&id)),
            None,
            "{jwk}"
        );
    }
}

/// An identifier that claims to be derived from a key is held to it. A document
/// that files one key under another key's thumbprint would otherwise let that
/// key answer to the other's `kid`.
#[test]
fn a_key_filed_under_another_keys_thumbprint_resolves_to_nothing() {
    let (key, other) = (public_key(), public_key());
    let jwk = KeyAlgorithm::Ed25519.published_jwk(&key);

    for id in [vm_id(&other), format!("{DID}#")] {
        let doc = document(&[
            json!({"id": id, "type": "JsonWebKey", "controller": DID, "publicKeyJwk": jwk}),
        ]);
        assert_eq!(resolve_verification_key(&doc, &naming(&id)), None, "{id}");
    }

    // An identifier with no fragment at all.
    let doc = document(&[
        json!({"id": DID, "type": "JsonWebKey", "controller": DID, "publicKeyJwk": jwk}),
    ]);
    assert_eq!(resolve_verification_key(&doc, &naming(DID)), None);
}

/// A document written by someone else names its keys as it likes. A fragment
/// that does not claim to be a thumbprint is a name, absolute or relative to the
/// document, and resolves like any other.
#[test]
fn a_fragment_that_is_not_a_thumbprint_is_taken_as_a_name() {
    let key = public_key();
    let jwk = KeyAlgorithm::Ed25519.published_jwk(&key);
    let absolute = format!("{DID}#key-1");

    for (id, reference) in [
        (absolute.as_str(), absolute.as_str()),
        ("#key-1", "#key-1"),
        ("#key-1", absolute.as_str()),
        (absolute.as_str(), "#key-1"),
    ] {
        let doc = json!({
            "id": DID,
            "verificationMethod": [
                {"id": id, "type": "JsonWebKey", "controller": DID, "publicKeyJwk": jwk},
            ],
            "assertionMethod": [reference],
        });
        assert_eq!(
            resolve_verification_key(&doc, &naming(&absolute)),
            Some(x(&key)),
            "id {id:?}, referenced as {reference:?}"
        );
    }
}

/// Controlled Identifiers v1.0 §3.3: the document must be the one the `kid`
/// names, and the method's controller must be that document. A document vouches
/// for its own keys only, so a method it lists under another DID, or for another
/// controller, is not one it can be relied on for.
#[test]
fn a_document_vouches_only_for_its_own_keys() {
    let key = public_key();
    let id = vm_id(&key);

    let mut filed_under_another_document = document(&[vm(&key)]);
    filed_under_another_document["id"] = json!("did:web:other.example");

    let mut controlled_by_another = vm(&key);
    controlled_by_another["controller"] = json!("did:web:other.example");

    let mut no_controller = vm(&key);
    no_controller
        .as_object_mut()
        .expect("object")
        .remove("controller");

    let mut no_document_id = document(&[vm(&key)]);
    no_document_id.as_object_mut().expect("object").remove("id");

    for doc in [
        filed_under_another_document,
        document(&[controlled_by_another]),
        document(&[no_controller]),
        no_document_id,
    ] {
        assert_eq!(resolve_verification_key(&doc, &naming(&id)), None, "{doc}");
    }
}

/// A key that declares what it is for is held to it: the JWS `alg` must be the
/// same string. A key that declares nothing accepts either spelling.
#[test]
fn a_jwk_that_declares_an_alg_binds_the_jws_to_it() {
    let key = public_key();
    let id = vm_id(&key);
    let with_alg = |declared: Option<&str>| {
        let mut jwk = KeyAlgorithm::Ed25519.published_jwk(&key);
        match declared {
            Some(alg) => jwk["alg"] = json!(alg),
            None => {
                jwk.as_object_mut().expect("object").remove("alg");
            }
        }
        document(&[json!({"id": id, "type": "JsonWebKey", "controller": DID, "publicKeyJwk": jwk})])
    };
    let resolves = |doc: &Value, alg: &str| {
        resolve_verification_key(doc, &token(&json!({"alg": alg, "kid": id}))).is_some()
    };

    let declares_eddsa = with_alg(Some("EdDSA"));
    assert!(resolves(&declares_eddsa, "EdDSA"));
    assert!(!resolves(&declares_eddsa, "Ed25519"));

    let declares_ed25519 = with_alg(Some("Ed25519"));
    assert!(resolves(&declares_ed25519, "Ed25519"));
    assert!(!resolves(&declares_ed25519, "EdDSA"));

    let declares_nothing = with_alg(None);
    assert!(resolves(&declares_nothing, "EdDSA"));
    assert!(resolves(&declares_nothing, "Ed25519"));
}
