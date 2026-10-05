//! JWS compact serialisation verifier — EdDSA/Ed25519, algorithm-pinned.

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use super::algorithm::KeyAlgorithm;

/// Verify an EdDSA compact JWS given a base64url-encoded public key.
///
/// Returns `Ok(true)` when the signature is valid, `Ok(false)` when it is not.
/// A protected header that names a critical extension (`crit`, RFC 7515 clause
/// 4.1.11) or any algorithm but `EdDSA` is refused the same way: `Ok(false)`.
/// Returns `Err` only on malformed input (bad base64, wrong key/sig length).
pub fn verify_jws(jws: &str, public_key_b64: &str) -> anyhow::Result<bool> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let parts: Vec<&str> = jws.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Ok(false);
    }

    // No key record here — the caller supplies raw Ed25519 key bytes — so the
    // algorithm is pinned rather than bound. Rejects `alg:none` and every
    // substitution downgrade.
    if !header_admissible(&b64, parts[0], KeyAlgorithm::Ed25519) {
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

/// Whether a JWS protected header may be acted on at all: it names no critical
/// extension, and its `alg` is exactly the algorithm the signing key is
/// recorded as using.
///
/// **`crit`.** RFC 7515 clause 4.1.11 says a JWS whose `crit` lists an
/// extension the recipient does not understand is invalid, and the recipient
/// must reject it. This crate understands none, so a `crit` of any shape is
/// refused rather than inspected. That also covers the shapes the clause forbids
/// a producer to emit (an empty list, a registered parameter, a repeated name),
/// which a recipient may treat as invalid. Ignoring `crit` instead would accept
/// a token its producer marked "do not process this unless you understand the
/// extension", for example one whose signing input is built differently.
///
/// **`alg`.** The header does not get a vote in which algorithm is used — it is
/// checked *against* the key. With one algorithm in the allowlist this is
/// equivalent to pinning; with two it is the difference between a verifier and
/// an algorithm-confusion oracle, which is why it is written this way now rather
/// than later. `false` for a malformed header, a missing `alg`, `alg:none`, or
/// anything outside the allowlist.
///
/// Every verification path in this crate calls this one function, so a check
/// added here cannot be skipped by a path that was written later.
pub(crate) fn header_admissible(
    b64: &base64::engine::general_purpose::GeneralPurpose,
    header_b64: &str,
    expected: KeyAlgorithm,
) -> bool {
    let Ok(bytes) = b64.decode(header_b64) else {
        return false;
    };
    let Ok(header) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return false;
    };
    if header.get("crit").is_some() {
        return false;
    }
    header
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .and_then(KeyAlgorithm::from_jose_alg)
        == Some(expected)
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
