//! JWS compact serialisation verifier — EdDSA/Ed25519, algorithm-pinned.

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};

use super::algorithm::{JWK_THUMBPRINT_URI_PREFIX, KeyAlgorithm};

/// Verify an EdDSA compact JWS given a base64url-encoded public key.
///
/// Returns `Ok(true)` when the signature is valid, `Ok(false)` when it is not.
/// A protected header that names a critical extension (`crit`, RFC 7515 clause
/// 4.1.11), or any algorithm but `EdDSA` and its RFC 9864 name `Ed25519`, is
/// refused the same way: `Ok(false)`.
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

/// The absolute form of an identifier inside the document `document_id`: a
/// fragment-only reference (`#…`) is resolved against the document, and anything
/// else is already absolute. Controlled Identifiers compares verification
/// methods by their absolute URL, and documents written by others do use the
/// relative form.
fn absolute(document_id: &str, id: &str) -> String {
    if id.starts_with('#') {
        format!("{document_id}{id}")
    } else {
        id.to_owned()
    }
}

/// Whether `assertionMethod` — the verification relationship that permits
/// signing credentials and passports — references the method `vm_id`.
fn is_assertion_method(did_document: &serde_json::Value, document_id: &str, vm_id: &str) -> bool {
    did_document
        .get("assertionMethod")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|refs| {
            refs.iter()
                .filter_map(serde_json::Value::as_str)
                .any(|r| absolute(document_id, r) == vm_id)
        })
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

/// Resolve the public key a JWS names, from its issuer's DID document: the
/// base64url Ed25519 `x` of the verification method the `kid` identifies.
///
/// The `kid` must be the absolute URL of a verification method: the document's
/// DID, `#`, and a fragment. The ones this crate issues end in the key's
/// thumbprint URI, `did:web:…#urn:ietf:params:oauth:jwk-thumbprint:sha-256:…`.
/// There is no fallback to a default key: a token that does not say which key
/// signed it is not verified against one chosen for it. `None` unless **all** of
/// these hold:
///
/// - the `kid` without its fragment is the document's own `id`, and a
///   verification method whose absolute `id` is the `kid` has that same
///   `controller`. These are the binding checks of the Controlled Identifiers
///   v1.0 retrieval algorithm (§3.3): a document vouches only for its own keys;
/// - `assertionMethod` references that method;
/// - its `publicKeyJwk` is an Ed25519 key (`kty: OKP`, `crv: Ed25519`);
/// - if the fragment is a thumbprint URI, it is that key's own. An identifier
///   that claims to be derived from the key is held to it, or a document could
///   file any key under another key's name. Other fragments, such as `#key-1`
///   in a document someone else wrote, are taken as the names they are;
/// - if the JWK declares an `alg`, the JWS `alg` is the same string. A key that
///   says what it is for is held to it.
///
/// The ids this crate issues are stable across rotation and revocation, so a
/// signature made before a rotation resolves to the key that made it. A revoked
/// key is absent from the document, and so resolves to nothing.
pub fn resolve_verification_key(did_document: &serde_json::Value, jws: &str) -> Option<String> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_bytes = b64.decode(jws.split('.').next()?).ok()?;
    let header: serde_json::Value = serde_json::from_slice(&header_bytes).ok()?;
    let kid = header.get("kid")?.as_str()?;
    let jws_alg = header.get("alg")?.as_str()?;

    let (document_url, fragment) = kid.split_once('#')?;
    let document_id = did_document.get("id")?.as_str()?;
    if fragment.is_empty() || document_url != document_id {
        return None;
    }

    let vm = did_document["verificationMethod"]
        .as_array()?
        .iter()
        .find(|vm| {
            vm.get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| absolute(document_id, id) == kid)
        })?;
    if vm.get("controller").and_then(serde_json::Value::as_str) != Some(document_id)
        || !is_assertion_method(did_document, document_id, kid)
    {
        return None;
    }

    let jwk = vm.get("publicKeyJwk")?;
    let x = jwk_ed25519_x(jwk)?;
    let raw = b64.decode(&x).ok()?;

    if fragment.starts_with(JWK_THUMBPRINT_URI_PREFIX)
        && fragment != KeyAlgorithm::Ed25519.thumbprint_uri(&raw)
    {
        return None;
    }
    if let Some(declared) = jwk.get("alg")
        && declared.as_str() != Some(jws_alg)
    {
        return None;
    }
    Some(x)
}
