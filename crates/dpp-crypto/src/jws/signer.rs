//! JWS compact serialisation signer — EdDSA over RFC 8785 (JCS) canonical bytes.

use base64::Engine;
use ed25519_dalek::Signer;
use serde_json::Value;

use crate::keystore::KeyStore;

/// Produces a compact JWS with algorithm `EdDSA` over the **RFC 8785 (JCS)
/// canonical** form of the JSON payload.
///
/// Signing over the canonical bytes (rather than incidental serde output) is
/// the content-binding contract: a verifier recomputes the canonical form of
/// the payload it holds and compares — see [`super::canonical`] and
/// [`crate::identity::local_service`].
///
/// The protected header includes a `kid` field set to the SHA-256 fingerprint
/// (hex) of the signing key's public bytes.  The verifier uses this `kid` to
/// select the correct verification method from the DID document, so that JWS
/// signatures remain verifiable after the operator rotates their key.
///
/// The curve is *not* emitted as a JOSE header parameter: RFC 8037 defines
/// `crv` as a JWK member, not a registered header parameter. It lives on the
/// DID document's `publicKeyJwk` instead, where it is the spec-correct place.
pub fn sign(store: &KeyStore, key_id: &str, payload: &Value) -> anyhow::Result<String> {
    let key = store.load_key(key_id)?;
    let header_json = format!(
        r#"{{"alg":"{}","kid":"{}"}}"#,
        super::algorithm::EDDSA_ALG,
        key.fingerprint
    );
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let canonical = super::canonical::canonicalize(payload)?;
    let header_b64 = b64.encode(header_json.as_bytes());
    let payload_b64 = b64.encode(&canonical);

    let signing_input = format!("{header_b64}.{payload_b64}");
    let signature = key.signing_key.sign(signing_input.as_bytes());
    let sig_b64 = b64.encode(signature.to_bytes());

    Ok(format!("{signing_input}.{sig_b64}"))
}

/// Verify a compact JWS produced by [`sign`] using the key store.
pub fn verify(store: &KeyStore, key_id: &str, jws: &str) -> anyhow::Result<bool> {
    let parts: Vec<&str> = jws.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Ok(false);
    }

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    // Pin the algorithm on the verify path too: reject `alg:none` and any
    // non-EdDSA header before touching the signature. Defence-in-depth for any
    // future multi-algorithm key store (today `verify_strict` is Ed25519-only).
    if !super::verifier::header_alg_is_eddsa(&b64, parts[0]) {
        return Ok(false);
    }

    let key = store.load_key(key_id)?;
    let signing_input = format!("{}.{}", parts[0], parts[1]);

    let sig_bytes = b64
        .decode(parts[2])
        .map_err(|e| anyhow::anyhow!("base64: {e}"))?;

    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid signature length"))?;

    let signature = ed25519_dalek::Signature::from_bytes(&sig_arr);

    // Strict verification (RFC 8032 §8) — see verifier::verify_jws.
    Ok(key
        .verifying_key
        .verify_strict(signing_input.as_bytes(), &signature)
        .is_ok())
}
