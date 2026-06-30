//! `did:web` DID document builder.
//!
//! Constructs a W3C DID document from the node's `KeyStore`: primary key first,
//! hygiene-archived keys as secondary verification methods, revoked keys excluded
//! so their signatures stop verifying.

use base64::Engine;
use serde_json::{Value, json};

use crate::keystore::KeyStore;

/// Build a `did:web` DID document for an issuer.
///
/// The DID is `did:web:{hostname}` (pathless; resolves to `/.well-known/did.json`).
///
/// The primary (current) key is listed first as `#key-1`.
/// Any archived keys are appended as secondary verification methods so that
/// signatures produced with rotated keys remain verifiable.
pub fn build_did_document(store: &KeyStore, base_url: &str, key_id: &str) -> anyhow::Result<Value> {
    if !store.has_key(key_id) {
        store.generate_key(key_id)?;
    }

    let current = store.load_key(key_id)?;

    let hostname = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    // Pathless form resolves to /.well-known/did.json per did:web spec.
    // Port colon must be %-encoded (RFC 3986 §3.3 path segment rule).
    let did = format!("did:web:{}", hostname.replace(':', "%3A"));

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let primary_vm_id = format!("{did}#key-1");
    let primary_pub_b64 = b64.encode(current.verifying_key.as_bytes());

    let mut verification_methods = vec![json!({
        "id": primary_vm_id,
        "type": "JsonWebKey2020",
        "controller": did,
        "publicKeyJwk": {
            "kty": "OKP",
            "crv": crate::jws::algorithm::ED25519_CRV,
            "x": primary_pub_b64
        }
    })];

    // Revoked keys are excluded entirely — neither a verification method nor an
    // assertionMethod — so signatures they produced no longer verify (Gap 7).
    let archived: Vec<_> = store
        .load_archived_keys(key_id)
        .into_iter()
        .filter(|k| !k.revoked)
        .collect();
    for (idx, archived_key) in archived.iter().enumerate() {
        let vm_id = format!("{did}#key-{}", idx + 2);
        let pub_b64 = b64.encode(archived_key.verifying_key.as_bytes());
        verification_methods.push(json!({
            "id": vm_id,
            "type": "JsonWebKey2020",
            "controller": did,
            "publicKeyJwk": {
                "kty": "OKP",
                "crv": crate::jws::algorithm::ED25519_CRV,
                "x": pub_b64
            }
        }));
    }

    let assertion_methods: Vec<String> = verification_methods
        .iter()
        .filter_map(|vm| vm["id"].as_str().map(String::from))
        .collect();

    let doc = json!({
        "@context": [
            "https://www.w3.org/ns/did/v1",
            "https://w3id.org/security/suites/jws-2020/v1"
        ],
        "id": did,
        "verificationMethod": verification_methods,
        "authentication": [primary_vm_id],
        "assertionMethod": assertion_methods
    });

    Ok(doc)
}
