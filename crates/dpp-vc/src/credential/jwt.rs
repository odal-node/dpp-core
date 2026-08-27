//! The signed envelope an access credential travels in: **VC-JWT**.
//!
//! A [`DppAccessCredential`] on its own is an unauthenticated document. Its
//! `issuer` is a string whoever produced it chose, so every check built on that
//! string — trust-registry lookups above all — means nothing until a signature
//! has been verified. Trust checking without signature verification is
//! decorative.
//!
//! # The wire format
//!
//! The credential travels as a **compact JWS** whose payload is the RFC 8785
//! (JCS) canonical form of the credential JSON. The credential model itself is
//! untouched: the envelope is external, and there is no `proof` member.
//!
//! - `alg` is `EdDSA`. `none` is rejected, and the algorithm is bound to the key
//!   record rather than read from the attacker-supplied header.
//! - `kid` is the signing key fingerprint, resolved against the issuer's
//!   `did:web` document.
//! - The payload is JCS-canonical, so one document has one byte sequence and a
//!   verifier never has to re-serialise anything to check a signature.
//!
//! # Why this rather than an embedded proof
//!
//! W3C Data Integrity would make the credential self-contained, which is the
//! obvious alternative. Four reasons went the other way, recorded here because a
//! wire format binds every issuer that ever produces one:
//!
//! 1. It reuses the verification path this workspace already has:
//!    `extract_kid_from_jws` → issuer DID document → `extract_key_by_fingerprint`
//!    → `verify_jws`.
//! 2. One canonicalisation scheme. A second would be a permanent review burden
//!    on the most security-sensitive code here, for no gain a verifier can use.
//! 3. Header-safe by construction: base64url, so it survives an HTTP header
//!    without escaping.
//! 4. It is the cheaper thing to change *from*. The ESPR Art. 11 credential
//!    implementing acts are unadopted, so this may need revisiting, and only the
//!    unwrap step would change rather than the credential model.
//!
//! # Order of operations
//!
//! [`authenticate_access_credential`] establishes only that the document is
//! authentic: signed by a key the claimed issuer publishes. It deliberately does
//! **not** check the validity window, the trust registry, or revocation. Those
//! belong to the composed verifiers this module sits beside
//! (`verify_credential_with_revocation_and_trust` and its siblings), and running
//! them first
//! would mean acting on attacker-chosen fields — fetching a status list from a
//! URL inside an unauthenticated document, above all.

use dpp_crypto::jws::signer;
use dpp_crypto::jws::verifier::{
    extract_key_by_fingerprint, extract_kid_from_jws, extract_primary_public_key, verify_jws,
};
use dpp_crypto::keystore::KeyStore;

use super::types::DppAccessCredential;
use super::verify::VerificationResult;

/// Sign a credential into its VC-JWT wire form, using `key_id` from `store`.
///
/// The payload is the JCS canonical form of `credential`, applied by
/// [`signer::sign`], so an issuer cannot accidentally sign a differently
/// ordered serialisation of the same document.
///
/// # Who should call this
///
/// Issuing an access credential is an **authority's** act, not a node's: the
/// signature says "this issuer vouches that the holder occupies this role". A
/// node signing its own access credentials has attested nothing to anyone. This
/// helper exists so that an issuer building on this crate produces the bytes a
/// verifier expects — not to suggest that issuing is a node's job.
///
/// # Errors
/// Propagates key-store and signing failures from [`signer::sign`], and fails if
/// the credential cannot be serialised to JSON.
pub fn sign_access_credential(
    credential: &DppAccessCredential,
    store: &KeyStore,
    key_id: &str,
) -> anyhow::Result<String> {
    let payload = serde_json::to_value(credential)?;
    signer::sign(store, key_id, &payload)
}

/// Authenticate a VC-JWT and return the credential it carries.
///
/// Establishes exactly one thing: the document was signed by a key published in
/// `issuer_did_document`, and it names that document's subject as its issuer.
/// Everything else is a separate step, deliberately.
///
/// `issuer_did_document` is a parameter rather than something this function
/// fetches, because resolving a DID is network I/O and this crate performs none.
/// The caller resolves it and **must resolve it from the credential's own
/// `issuer` value**. This function re-checks that binding against the document
/// it was handed, so a caller that resolves the wrong document is refused rather
/// than silently trusted.
///
/// # Errors
/// [`VerificationResult::InvalidSignature`] when the JWS is malformed, no key in
/// the document matches, the signature does not verify, or the document belongs
/// to a different issuer than the credential claims.
/// [`VerificationResult::MalformedCredential`] when the payload is not a
/// credential.
pub fn authenticate_access_credential(
    jws: &str,
    issuer_did_document: &serde_json::Value,
) -> Result<DppAccessCredential, VerificationResult> {
    use base64::Engine as _;

    // The key the JWS names, falling back to the document's primary key for an
    // issuer that publishes one verification method and no `kid`.
    let key = match extract_kid_from_jws(jws) {
        Some(kid) => extract_key_by_fingerprint(issuer_did_document, &kid),
        None => extract_primary_public_key(issuer_did_document),
    }
    .ok_or_else(|| {
        VerificationResult::InvalidSignature(
            "issuer DID document publishes no key matching this credential".into(),
        )
    })?;

    match verify_jws(jws, &key) {
        Ok(true) => {}
        Ok(false) => {
            return Err(VerificationResult::InvalidSignature(
                "signature did not verify against the issuer's published key".into(),
            ));
        }
        Err(e) => return Err(VerificationResult::InvalidSignature(e.to_string())),
    }

    let payload = jws
        .split('.')
        .nth(1)
        .ok_or_else(|| VerificationResult::InvalidSignature("not a compact JWS".into()))?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| {
            VerificationResult::MalformedCredential(format!("payload is not base64url: {e}"))
        })?;
    let credential: DppAccessCredential = serde_json::from_slice(&bytes).map_err(|e| {
        VerificationResult::MalformedCredential(format!("payload is not a credential: {e}"))
    })?;

    // Bind the credential to the document it was checked against. Without this, a
    // caller that resolved the wrong DID — or was handed one — would accept a
    // credential signed by an issuer it never names, and the trust registry would
    // then be consulted about the *claimed* issuer rather than the one that
    // actually signed.
    let document_subject = issuer_did_document
        .get("id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if document_subject != credential.issuer {
        return Err(VerificationResult::InvalidSignature(format!(
            "credential names issuer {} but was checked against the DID document for {document_subject}",
            credential.issuer
        )));
    }

    Ok(credential)
}
