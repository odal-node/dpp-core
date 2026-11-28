//! Building a did:web identifier and its DID document.

use serde_json::{Map, Value, json};

use dpp_crypto::keystore::{KeyBackend, PublicKeyInfo};

/// The DID of the issuer whose documents are served from `base_url`.
///
/// `did:web:{hostname}`, pathless, so it resolves to `/.well-known/did.json`. Only
/// the URL's authority is used: a trailing slash, path, query or fragment would
/// otherwise land inside the DID, where `/`, `?` and `#` begin a DID URL's own
/// components rather than belonging to the identifier. A port's colon is
/// percent-encoded, because it would otherwise read as a path separator.
pub fn did_for(base_url: &str) -> String {
    let rest = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    format!("did:web:{}", authority.replace(':', "%3A"))
}

/// Build a `did:web` DID document for an issuer.
///
/// The primary (current) key is listed first. Any archived keys that have not
/// been revoked are appended as secondary verification methods so that
/// signatures produced with rotated keys remain verifiable. Every method is
/// listed in `assertionMethod`; only the primary is in `authentication`.
///
/// This function only reads from `store`. Callers must provision the issuer's
/// key during application startup before serving this document.
///
/// **A revoked key is never listed, the current one included**, so the document
/// publishes the same keys as [`build_issuer_metadata`]. A store's own calls never
/// leave its current key revoked, but a store whose file says so opens: one written
/// before key stores carried an integrity tag, sealed by
/// [`KeyStore::open_and_migrate`](dpp_crypto::keystore::KeyStore::open_and_migrate). Such a key signs nothing either
/// ([`KeyRevoked`](dpp_crypto::keystore::KeyRevoked)).
///
/// With no usable current key, `authentication` is left out rather than empty: DID Core
/// 1.0 §5.3.1 says that if it is present, "the associated value MUST be a set of
/// one or more verification methods", and §5.3.2 says the same of `assertionMethod`.
/// If the current key is missing, this function returns an error. When the current
/// key and all archived keys are revoked, `verificationMethod` and `assertionMethod`
/// are left out, and the document holds its `@context` and `id` alone.
///
/// [`build_issuer_metadata`]: crate::sd_jwt_vc::build_issuer_metadata
pub fn build_did_document(
    store: &dyn KeyBackend,
    base_url: &str,
    key_id: &str,
) -> anyhow::Result<Value> {
    // Only the public key is needed to build a DID document — read it
    // directly from the store's plaintext `verifying_key_hex` rather than
    // decrypting the private signing key just to derive it back.
    let current = store
        .public_key(key_id)
        .ok_or_else(|| anyhow::anyhow!("no key found for {key_id}"))?;

    let did = did_for(base_url);
    let mut verification_methods = Vec::new();
    let mut authentication = None;
    if !current.revoked {
        let primary = verification_method(&did, &current)?;
        authentication = Some(primary["id"].clone());
        verification_methods.push(primary);
    }

    // Revoked keys are excluded entirely — neither a verification method nor an
    // assertionMethod — so signatures they produced no longer verify.
    for archived_key in store
        .archived_public_keys(key_id)
        .into_iter()
        .filter(|k| !k.revoked)
    {
        verification_methods.push(verification_method(&did, &archived_key)?);
    }

    let mut doc = Map::new();
    doc.insert(
        "@context".to_owned(),
        json!([
            "https://www.w3.org/ns/did/v1",
            "https://www.w3.org/ns/cid/v1"
        ]),
    );
    doc.insert("id".to_owned(), Value::String(did));
    if let Some(primary) = authentication {
        doc.insert("authentication".to_owned(), json!([primary]));
    }
    if !verification_methods.is_empty() {
        let assertion_methods: Vec<Value> = verification_methods
            .iter()
            .map(|vm| vm["id"].clone())
            .collect();
        doc.insert(
            "assertionMethod".to_owned(),
            Value::Array(assertion_methods),
        );
        doc.insert(
            "verificationMethod".to_owned(),
            Value::Array(verification_methods),
        );
    }

    Ok(Value::Object(doc))
}

/// One verification method: a `JsonWebKey` identified by its own thumbprint.
fn verification_method(did: &str, key: &PublicKeyInfo) -> anyhow::Result<Value> {
    let public_key = key.public_key()?;
    Ok(json!({
        "id": format!("{did}#{}", public_key.thumbprint_uri()),
        "type": "JsonWebKey",
        "controller": did,
        // The JWK shape comes from the algorithm recorded on the key, not from
        // an assumption here — `kty` and the parameter set differ per
        // algorithm, and `dpp-crypto` is the crate that knows which is which.
        // Per key, so a rotation across algorithms produces a document carrying
        // both shapes rather than one mislabelled.
        "publicKeyJwk": public_key.published_jwk()
    }))
}
