//! `did:web` DID document builder.
//!
//! Constructs a W3C DID document from the node's `KeyStore`: primary key first,
//! hygiene-archived keys as secondary verification methods, revoked keys excluded
//! so their signatures stop verifying.

use serde_json::{Value, json};

use dpp_crypto::keystore::KeyStore;

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

    // Only the public key is needed to build a DID document — read it
    // directly from the store's plaintext `verifying_key_hex` rather than
    // decrypting the private signing key just to derive it back.
    let current = store
        .public_key(key_id)
        .ok_or_else(|| anyhow::anyhow!("no key found for {key_id}"))?;

    let hostname = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    // Pathless form resolves to /.well-known/did.json per did:web spec.
    // Port colon must be %-encoded (RFC 3986 §3.3 path segment rule).
    let did = format!("did:web:{}", hostname.replace(':', "%3A"));

    let primary_vm_id = format!("{did}#key-1");

    let mut verification_methods = vec![json!({
        "id": primary_vm_id,
        "type": "JsonWebKey2020",
        "controller": did,
        // The JWK shape comes from the algorithm recorded on the key, not from
        // an assumption here — `kty` and the parameter set differ per
        // algorithm, and `dpp-crypto` is the crate that knows which is which.
        "publicKeyJwk": current
            .algorithm
            .public_key_jwk(&hex::decode(&current.verifying_key_hex)?)
    })];

    // Revoked keys are excluded entirely — neither a verification method nor an
    // assertionMethod — so signatures they produced no longer verify (Gap 7).
    let archived: Vec<_> = store
        .archived_public_keys(key_id)
        .into_iter()
        .filter(|k| !k.revoked)
        .collect();
    for (idx, archived_key) in archived.iter().enumerate() {
        let vm_id = format!("{did}#key-{}", idx + 2);
        verification_methods.push(json!({
            "id": vm_id,
            "type": "JsonWebKey2020",
            "controller": did,
            // Per archived key, so a rotation across algorithms produces a
            // document carrying both shapes rather than one mislabelled.
            "publicKeyJwk": archived_key
                .algorithm
                .public_key_jwk(&hex::decode(&archived_key.verifying_key_hex)?)
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
