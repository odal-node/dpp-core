//! JWS compact serialisation verifier, algorithm-bound: the key decides the
//! algorithm and the header may only confirm it.

use base64::Engine;

use super::algorithm::{JWK_THUMBPRINT_URI_PREFIX, KeyAlgorithm};
use super::error::{JwsError, SignatureLengthError};
use super::header_refusal::HeaderRefusal;
use super::public_key::PublicKey;

/// Verify a compact JWS under `key`.
///
/// Returns `Ok(())` when the signature is valid, or a [`JwsError`] naming why it
/// was refused.
pub fn verify_jws(jws: &str, key: &PublicKey) -> Result<(), JwsError> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let mut parts = jws.split('.');
    let (Some(protected), Some(payload), Some(signature)) =
        (parts.next(), parts.next(), parts.next())
    else {
        return Err(JwsError::MalformedCompact);
    };
    if parts.next().is_some() {
        return Err(JwsError::MalformedCompact);
    }

    // The key names its algorithm; the header may only confirm it. Rejects
    // `alg:none` and every substitution downgrade.
    if let Some(refusal) = header_refusal(jws, key.algorithm()) {
        return Err(JwsError::HeaderRefused(refusal));
    }

    let signing_input = format!("{protected}.{payload}");
    let sig_bytes = b64.decode(signature).map_err(|_| JwsError::BadBase64)?;
    let expected = key.algorithm().signature_len();
    if sig_bytes.len() != expected {
        return Err(JwsError::SignatureLength(SignatureLengthError {
            algorithm: key.algorithm(),
            expected,
            actual: sig_bytes.len(),
        }));
    }

    if key.verify(signing_input.as_bytes(), &sig_bytes) {
        Ok(())
    } else {
        Err(JwsError::SignatureDoesNotVerify)
    }
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

/// Whether a JWS protected header may be acted on at all: [`header_refusal`] has
/// nothing to say about it.
///
/// Every verification path in this crate calls this one function, or
/// [`header_refusal`] that it is defined by, so a check added there cannot be
/// skipped by a path that was written later.
pub(crate) fn header_admissible(jws: &str, expected: KeyAlgorithm) -> bool {
    header_refusal(jws, expected).is_none()
}

/// Why the protected header of the compact JWS `jws` may not be acted on, or
/// `None` when it may: it names no critical extension, and its `alg` is exactly
/// the algorithm the signing key is recorded as using. Only the first segment
/// is read.
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
/// than later. Refused for a missing `alg`, `alg:none`, or anything outside the
/// allowlist.
///
/// [`verify_jws`] returns the refusal as [`JwsError::HeaderRefused`]. This helper
/// remains useful when a caller needs to inspect a protected header without
/// verifying its signature.
#[must_use]
pub fn header_refusal(jws: &str, expected: KeyAlgorithm) -> Option<HeaderRefusal> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_b64 = jws.split('.').next().unwrap_or_default();
    let Ok(bytes) = b64.decode(header_b64) else {
        return Some(HeaderRefusal::Unreadable);
    };
    let Ok(serde_json::Value::Object(header)) = serde_json::from_slice(&bytes) else {
        return Some(HeaderRefusal::Unreadable);
    };
    if header.contains_key("crit") {
        return Some(HeaderRefusal::Critical);
    }
    let alg = header.get("alg").and_then(serde_json::Value::as_str);
    if alg.and_then(KeyAlgorithm::from_jose_alg) == Some(expected) {
        None
    } else {
        Some(HeaderRefusal::Algorithm {
            found: alg.map(str::to_owned),
        })
    }
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

/// Resolve the public key a JWS names, from its issuer's DID document: the key
/// of the verification method the `kid` identifies, with its algorithm.
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
/// - its `publicKeyJwk` is an Ed25519 key (`OKP`, `crv: Ed25519`) or an EC key on
///   P-256, P-384 or P-521 (`EC`, `crv` naming that curve), on its curve;
/// - if the fragment is a thumbprint URI, it is that key's own. An identifier
///   that claims to be derived from the key is held to it, or a document could
///   file any key under another key's name. Other fragments, such as `#key-1`
///   in a document someone else wrote, are taken as the names they are;
/// - if the JWK declares an `alg`, the JWS `alg` is the same string. A key that
///   says what it is for is held to it;
/// - the JWS `alg` names the key's algorithm (`EdDSA` or `Ed25519` for an Ed25519
///   key, `ES256`, `ES384` or `ES512` for a P-256, P-384 or P-521 key), even when the
///   JWK declares no `alg`.
///
/// The ids this crate issues are stable across rotation and revocation, so a
/// signature made before a rotation resolves to the key that made it. A revoked
/// key is absent from the document, and so resolves to nothing.
pub fn resolve_verification_key(did_document: &serde_json::Value, jws: &str) -> Option<PublicKey> {
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
    let key = PublicKey::from_jwk(jwk)?;

    if fragment.starts_with(JWK_THUMBPRINT_URI_PREFIX) && fragment != key.thumbprint_uri() {
        return None;
    }
    if let Some(declared) = jwk.get("alg")
        && declared.as_str() != Some(jws_alg)
    {
        return None;
    }
    // A JWK that declares no `alg` still has one: its key type's. The header is
    // checked against it again by `verify_jws`; refusing here as well keeps a
    // resolver caller from acting on a key the token's `alg` contradicts.
    if KeyAlgorithm::from_jose_alg(jws_alg) != Some(key.algorithm()) {
        return None;
    }
    Some(key)
}
