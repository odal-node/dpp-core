//! JWT VC Issuer Metadata — the document that replaces `did:web` for this
//! credential type.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use dpp_crypto::keystore::{KeyStore, PublicKeyInfo};

/// The well-known path a verifier fetches, from
/// draft-ietf-oauth-sd-jwt-vc-19 clause 4: inserted between the host and path
/// components of the `iss` value.
pub const WELL_KNOWN_PATH: &str = "/.well-known/jwt-vc-issuer";

/// Build the JWT VC Issuer Metadata configuration for this node.
///
/// # Why this exists alongside the DID document
///
/// This crate already publishes a `did:web` document, and that stays: the W3C
/// credential path and the UNTP door both resolve keys through it. SD-JWT VC
/// does not. Clause 2.5 of the profile defines exactly two key-discovery
/// mechanisms — this document, and an inline `x5c` certificate chain — and
/// **names no DID method at all**. A verifier built to the profile will not
/// resolve `did:web`, so a credential that offered only a DID would be
/// unverifiable by a conformant consumer.
///
/// The `x5c` route is the other half of clause 2.5 and is not implemented here:
/// it makes the issuer *the subject of an end-entity certificate*, which is a
/// procurement fact rather than a code one, and the credential would then be
/// making a claim about a certificate holder that nothing in this crate can
/// substantiate.
///
/// # Shape
///
/// Clause 4.2 requires `issuer`, identical to the credential's `iss` claim, plus
/// **exactly one** of `jwks` or `jwks_uri` — "but not both". Keys are embedded
/// by value: a `jwks_uri` would be a second document to serve, a second URL to
/// keep resolving for the life of a passport, and a second thing to get wrong,
/// for no benefit at this key count.
///
/// Archived keys are included and revoked keys are excluded — **including the
/// current one** — exactly as the DID document does it: a credential signed
/// before a rotation must keep verifying, and one signed with a revoked key must
/// stop. Returns `None` when nothing usable is left, rather than an empty key
/// set, which would assert that this issuer signs nothing.
pub fn build_issuer_metadata(store: &KeyStore, issuer: &str, key_id: &str) -> Option<Value> {
    let current = store.public_key(key_id)?;

    // 🚨 The current key is filtered on `revoked` too, and it was not.
    //
    // The doc above says revoked keys are excluded; the code excluded them only
    // from the *archived* list, and published whatever `public_key` returned.
    // `revoked` is persisted on the record, so a store opened or migrated with a
    // revoked current key would publish it here — and a verifier trusting this
    // JWKS would go on accepting signatures from the one key an operator has
    // said to stop trusting. That is the exact failure revocation exists to
    // prevent, reached by the metadata document meant to convey it.
    let mut keys: Vec<Value> = Vec::new();
    if !current.revoked {
        keys.push(jwk(&current)?);
    }
    keys.extend(
        store
            .archived_public_keys(key_id)
            .iter()
            .filter(|k| !k.revoked)
            .filter_map(jwk),
    );

    // No usable key is not the same as an empty key set. A JWKS with `keys: []`
    // reads as "this issuer signs nothing", which a verifier may cache; absent
    // metadata reads as "ask again". Fail closed by saying nothing.
    if keys.is_empty() {
        return None;
    }

    Some(json!({
        "issuer": issuer,
        "jwks": { "keys": keys },
    }))
}

/// One JWK, carrying the `kid` the Issuer-signed JWT's header names.
///
/// Clause 4.2: *"It is RECOMMENDED that the Issuer-signed JWT contains a `kid`
/// JWT header parameter that can be used to look up the public key in the JWK
/// Set."* The signer emits the key's fingerprint as `kid`, so the same value has
/// to appear here or the recommendation is met in name only — a verifier holding
/// two keys would have no way to pick.
fn jwk(key: &PublicKeyInfo) -> Option<Value> {
    let bytes = hex::decode(&key.verifying_key_hex).ok()?;
    let mut jwk = key.algorithm.public_key_jwk(&bytes);
    let object = jwk.as_object_mut()?;
    // The same derivation the key store uses for `KeyEntry::fingerprint`, which
    // is what `jws::sign_typed` writes into the header.
    object.insert(
        "kid".to_owned(),
        Value::String(hex::encode(Sha256::digest(&bytes))),
    );
    // RFC 7517 clause 4.2/4.4: these keys verify signatures and nothing else.
    object.insert("use".to_owned(), Value::String("sig".to_owned()));
    object.insert(
        "alg".to_owned(),
        Value::String(key.algorithm.jose_alg().to_owned()),
    );
    Some(jwk)
}
