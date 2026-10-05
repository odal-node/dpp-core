//! `did:web` DID document builder.
//!
//! Constructs a W3C DID document from an issuer's `KeyStore`: primary key first,
//! hygiene-archived keys as secondary verification methods, revoked keys excluded
//! so their signatures stop verifying.
//!
//! # A key's identifier is its thumbprint
//!
//! A verification method is identified by `did:web:<host>#<thumbprint URI>`, the
//! thumbprint being the RFC 7638 one of the key's JWK in the RFC 9278 URI form.
//! The identifier is a function of the key and nothing else, so it is the same
//! before and after a rotation and for every reader. A position in a list would
//! not be: the identifiers of archived keys used to be renumbered whenever a
//! revoked one dropped out, so a token that named `#key-1` named a different key
//! after the next rotation. The `kid` in a signed token's protected header is
//! fixed when it is issued, so the identifier it names has to be fixed too.
//!
//! The verification methods are of type `JsonWebKey`, defined by W3C Controlled
//! Identifiers v1.0 together with `Multikey`. DID 1.1 defines its key properties
//! by reference to that specification.

use serde_json::{Value, json};

use dpp_crypto::keystore::{KeyStore, PublicKeyInfo};

/// The DID of the issuer whose documents are served from `base_url`.
///
/// `did:web:{hostname}`, pathless, so it resolves to `/.well-known/did.json`. A
/// port's colon is percent-encoded, because it would otherwise read as a path
/// separator.
pub fn did_for(base_url: &str) -> String {
    let hostname = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    format!("did:web:{}", hostname.replace(':', "%3A"))
}

/// Build a `did:web` DID document for an issuer.
///
/// The primary (current) key is listed first. Any archived keys that have not
/// been revoked are appended as secondary verification methods so that
/// signatures produced with rotated keys remain verifiable. Every method is
/// listed in `assertionMethod`; only the primary is in `authentication`.
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

    let did = did_for(base_url);
    let primary = verification_method(&did, &current)?;
    let primary_vm_id = primary["id"].clone();

    let mut verification_methods = vec![primary];

    // Revoked keys are excluded entirely — neither a verification method nor an
    // assertionMethod — so signatures they produced no longer verify.
    for archived_key in store
        .archived_public_keys(key_id)
        .into_iter()
        .filter(|k| !k.revoked)
    {
        verification_methods.push(verification_method(&did, &archived_key)?);
    }

    let assertion_methods: Vec<&Value> = verification_methods.iter().map(|vm| &vm["id"]).collect();

    let doc = json!({
        "@context": [
            "https://www.w3.org/ns/did/v1",
            "https://www.w3.org/ns/cid/v1"
        ],
        "id": did,
        "verificationMethod": verification_methods,
        "authentication": [primary_vm_id],
        "assertionMethod": assertion_methods
    });

    Ok(doc)
}

/// One verification method: a `JsonWebKey` identified by its own thumbprint.
fn verification_method(did: &str, key: &PublicKeyInfo) -> anyhow::Result<Value> {
    let public_key = hex::decode(&key.verifying_key_hex)?;
    Ok(json!({
        "id": format!("{did}#{}", key.algorithm.thumbprint_uri(&public_key)),
        "type": "JsonWebKey",
        "controller": did,
        // The JWK shape comes from the algorithm recorded on the key, not from
        // an assumption here — `kty` and the parameter set differ per
        // algorithm, and `dpp-crypto` is the crate that knows which is which.
        // Per key, so a rotation across algorithms produces a document carrying
        // both shapes rather than one mislabelled.
        "publicKeyJwk": key.algorithm.published_jwk(&public_key)
    }))
}
