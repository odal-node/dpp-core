//! The vendored EdDSA verifier and DID-document key-extraction helpers.

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// Verify an EdDSA compact JWS given a base64url-encoded public key.
///
/// Returns `Ok(true)` when the signature is valid, `Ok(false)` when it is not.
/// Returns `Err` only on malformed input (bad base64, wrong key/sig length).
pub fn verify_jws(jws: &str, public_key_b64: &str) -> Result<bool, VerifyError> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let parts: Vec<&str> = jws.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Ok(false);
    }

    // Pin the algorithm: only EdDSA is accepted. Reject `alg:none` and any other
    // algorithm in the header to prevent algorithm-substitution downgrade.
    if !header_alg_is_eddsa(&b64, parts[0]) {
        return Ok(false);
    }

    let signing_input = format!("{}.{}", parts[0], parts[1]);

    let sig_bytes = b64
        .decode(parts[2])
        .map_err(|e| VerifyError::Malformed(format!("base64 signature: {e}")))?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| VerifyError::Malformed("Ed25519 signature must be 64 bytes".into()))?;
    let signature = Signature::from_bytes(&sig_arr);

    let key_bytes = b64
        .decode(public_key_b64)
        .map_err(|e| VerifyError::Malformed(format!("base64 public key: {e}")))?;
    let key_arr: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| VerifyError::Malformed("Ed25519 public key must be 32 bytes".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&key_arr)
        .map_err(|e| VerifyError::Malformed(format!("invalid key: {e}")))?;

    // Strict verification (RFC 8032 §8): rejects the signature-malleability /
    // small-order/cofactor edge cases that the non-strict `verify` admits —
    // undesirable when the signature is the trust anchor.
    Ok(verifying_key
        .verify_strict(signing_input.as_bytes(), &signature)
        .is_ok())
}

/// Error from a malformed JWS or key — distinct from "signature did not verify",
/// which is a plain `Ok(false)`.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("malformed input: {0}")]
    Malformed(String),
}

/// Extract the base64url-encoded `x` from a JWK, but only if it is a genuine
/// Ed25519 key (`kty:"OKP"`, `crv:"Ed25519"`). Returns `None` for any other
/// key type/curve so a malformed or wrong-curve JWK can't be mis-selected.
fn jwk_ed25519_x(jwk: &serde_json::Value) -> Option<String> {
    if jwk.get("kty")?.as_str()? != "OKP" {
        return None;
    }
    if jwk.get("crv")?.as_str()? != "Ed25519" {
        return None;
    }
    jwk.get("x")?.as_str().map(String::from)
}

/// IDs the DID document authorizes via `assertionMethod` — the verification
/// relationship that permits signing credentials/passports.
fn assertion_method_ids(did_document: &serde_json::Value) -> Vec<String> {
    did_document
        .get("assertionMethod")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether a verification-method entry is referenced by `assertionMethod`.
fn vm_is_assertion_authorized(vm: &serde_json::Value, authorized: &[String]) -> bool {
    vm.get("id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| authorized.iter().any(|a| a == id))
}

/// Decode the JWS protected header and confirm `alg == "EdDSA"`.
fn header_alg_is_eddsa(
    b64: &base64::engine::general_purpose::GeneralPurpose,
    header_b64: &str,
) -> bool {
    b64.decode(header_b64)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|h| h.get("alg").and_then(|v| v.as_str()).map(str::to_owned))
        .is_some_and(|alg| alg == "EdDSA")
}

/// Extract the base64url-encoded primary Ed25519 public key (`x`) from a DID document.
///
/// Looks at `verificationMethod[0].publicKeyJwk.x`.
pub fn extract_primary_public_key(did_document: &serde_json::Value) -> Option<String> {
    let authorized = assertion_method_ids(did_document);
    did_document["verificationMethod"]
        .as_array()?
        .iter()
        .find_map(|vm| {
            if vm_is_assertion_authorized(vm, &authorized) {
                jwk_ed25519_x(vm.get("publicKeyJwk")?)
            } else {
                None
            }
        })
}

/// Extract the `kid` field from the JWS protected header.
///
/// Returns `None` if the JWS is malformed or the header contains no `kid`.
pub fn extract_kid_from_jws(jws: &str) -> Option<String> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_b64 = jws.split('.').next()?;
    let header_bytes = b64.decode(header_b64).ok()?;
    let header: serde_json::Value = serde_json::from_slice(&header_bytes).ok()?;
    header.get("kid")?.as_str().map(String::from)
}

/// Find the base64url-encoded Ed25519 public key (`x`) in a DID document
/// whose SHA-256 fingerprint (hex) matches `kid`.
///
/// The `kid` embedded in the JWS protected header is
/// `hex::encode(Sha256::digest(verifying_key_bytes))`. This iterates all
/// `verificationMethod` entries and returns the `x` value of the first one
/// whose decoded public key produces the same fingerprint — allowing
/// verification against any rotation-archived key.
pub fn extract_key_by_fingerprint(did_document: &serde_json::Value, kid: &str) -> Option<String> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let authorized = assertion_method_ids(did_document);
    did_document["verificationMethod"]
        .as_array()?
        .iter()
        .find_map(|vm| {
            if !vm_is_assertion_authorized(vm, &authorized) {
                return None;
            }
            let x = jwk_ed25519_x(vm.get("publicKeyJwk")?)?;
            let raw = b64.decode(&x).ok()?;
            if hex::encode(Sha256::digest(&raw)) == kid {
                Some(x)
            } else {
                None
            }
        })
}

/// Resolve the public key to verify `jws` against, from a DID document: try
/// the `kid`-fingerprint match first (supports rotation-archived keys), then
/// fall back to the primary key for JWS tokens signed before `kid` was added.
pub fn resolve_public_key(jws: &str, did_document: &serde_json::Value) -> Option<String> {
    if let Some(kid) = extract_kid_from_jws(jws)
        && let Some(key) = extract_key_by_fingerprint(did_document, &kid)
    {
        return Some(key);
    }
    extract_primary_public_key(did_document)
}

/// Decode the payload segment of a compact JWS to raw bytes (post-base64,
/// pre-JSON-parse).
pub fn decode_payload_bytes(jws: &str) -> Result<Vec<u8>, VerifyError> {
    let payload_b64 = jws
        .split('.')
        .nth(1)
        .ok_or_else(|| VerifyError::Malformed("JWS has no payload segment".into()))?;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| VerifyError::Malformed(format!("payload base64: {e}")))
}

/// Verify both that `jws` is validly signed under `public_key_b64` **and**
/// that its embedded payload is exactly the JCS-canonical bytes of
/// `expected`.
///
/// The plain [`verify_jws`] only checks the signature is internally
/// consistent — it says nothing about *what* was signed. Every signer in
/// this system embeds `base64url(JCS(payload))` as the payload segment
/// (`dpp-crypto/src/jws/signer.rs`), so content-binding means recomputing
/// those same canonical bytes and comparing. Without this step a validly
/// signed JWS over the *wrong* content would incorrectly verify.
pub fn verify_jws_content(
    jws: &str,
    public_key_b64: &str,
    expected: &serde_json::Value,
) -> Result<bool, VerifyError> {
    if !verify_jws(jws, public_key_b64)? {
        return Ok(false);
    }
    let actual = decode_payload_bytes(jws)?;
    let expected_bytes = serde_jcs::to_vec(expected)
        .map_err(|e| VerifyError::Malformed(format!("JCS canonicalisation: {e}")))?;
    Ok(actual == expected_bytes)
}
