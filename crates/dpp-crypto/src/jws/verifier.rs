//! JWS compact serialisation verifier — EdDSA/Ed25519, algorithm-pinned.

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// Verify an EdDSA compact JWS given a base64url-encoded public key.
///
/// Returns `Ok(true)` when the signature is valid, `Ok(false)` when it is not.
/// Returns `Err` only on malformed input (bad base64, wrong key/sig length).
pub fn verify_jws(jws: &str, public_key_b64: &str) -> anyhow::Result<bool> {
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
        .map_err(|e| anyhow::anyhow!("base64 signature: {e}"))?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Ed25519 signature must be 64 bytes"))?;
    let signature = Signature::from_bytes(&sig_arr);

    let key_bytes = b64
        .decode(public_key_b64)
        .map_err(|e| anyhow::anyhow!("base64 public key: {e}"))?;
    let key_arr: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Ed25519 public key must be 32 bytes"))?;
    let verifying_key =
        VerifyingKey::from_bytes(&key_arr).map_err(|e| anyhow::anyhow!("invalid key: {e}"))?;

    // Strict verification (RFC 8032 §8): rejects the signature-malleability /
    // small-order/cofactor edge cases that the non-strict `verify` admits —
    // undesirable when the signature is the trust anchor.
    Ok(verifying_key
        .verify_strict(signing_input.as_bytes(), &signature)
        .is_ok())
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
pub(crate) fn header_alg_is_eddsa(
    b64: &base64::engine::general_purpose::GeneralPurpose,
    header_b64: &str,
) -> bool {
    b64.decode(header_b64)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|h| h.get("alg").and_then(|v| v.as_str()).map(str::to_owned))
        .map(|alg| super::algorithm::is_allowed_alg(&alg))
        .unwrap_or(false)
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
/// Old JWS tokens produced before the kid-header change will return `None`
/// and callers should fall back to `extract_primary_public_key`.
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
/// The `kid` embedded in the JWS protected header by `signer::sign` is
/// `hex::encode(Sha256::digest(verifying_key_bytes))`.  This function
/// iterates all `verificationMethod` entries and returns the `x` value of
/// the first one whose decoded public key produces the same fingerprint —
/// allowing verification against any rotation-archived key.
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
