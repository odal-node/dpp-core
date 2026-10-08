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
//!   record rather than read from the attacker-supplied header. A verifier also
//!   accepts `Ed25519`, the RFC 9864 name for the same operation.
//! - `kid` is the absolute DID URL of the signing key's verification method in the
//!   issuer's `did:web` document: the issuer's DID, then `#`, then the key's
//!   thumbprint URI. It is required, and it is stable across rotation.
//! - `typ` is `vc+jwt` and `cty` is `vc`, as VC-JOSE-COSE asks. A verifier
//!   requires the `typ`, so a token of another kind signed by the same key, a
//!   passport proof or an SD-JWT VC, is not taken for a credential: the explicit
//!   typing RFC 8725 §3.11 recommends. A `cty`, when present, must say `vc`.
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
//!    issuer DID document → `resolve_verification_key` → `verify_jws`.
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
use dpp_crypto::jws::verifier::{extract_kid_from_jws, resolve_verification_key, verify_jws};
use dpp_crypto::keystore::KeyStore;

use super::types::DppAccessCredential;
use super::verify::VerificationResult;

/// The `typ` of a VC-JWT, and the `cty` naming what it carries (VC-JOSE-COSE).
const TYP: &str = "vc+jwt";
const CTY: &str = "vc";

/// Sign a credential into its VC-JWT wire form, using `key_id` from `store`.
///
/// The payload is the JCS canonical form of `credential`, applied by
/// [`signer::sign_typed`], so an issuer cannot accidentally sign a differently
/// ordered serialisation of the same document. The header carries
/// `typ: vc+jwt` and `cty: vc`.
///
/// # Who should call this
///
/// Issuing an access credential is an **authority's** act, not a node's: the
/// signature says "this issuer vouches that the holder occupies this role". A
/// node signing its own access credentials has attested nothing to anyone. This
/// helper exists so that an issuer building on this crate produces the bytes a
/// verifier expects — not to suggest that issuing is a node's job.
///
/// # The `kid`
///
/// The credential has no `iss` claim, so the token's `kid` has to be the absolute
/// URL of the verification method (VC-JOSE-COSE, key discovery by `kid`). It is
/// the credential's `issuer`, which is the issuer's DID, followed by the signing
/// key's thumbprint URI. That is the identifier [`build_did_document`] gives the
/// key, so it resolves in the issuer's document for as long as the key is in it.
///
/// # Errors
/// Propagates key-store and signing failures from [`signer::sign_typed`], fails
/// if `key_id` names no key, and fails if the credential cannot be serialised to
/// JSON.
///
/// [`build_did_document`]: crate::did_builder::build_did_document
pub fn sign_access_credential(
    credential: &DppAccessCredential,
    store: &KeyStore,
    key_id: &str,
) -> anyhow::Result<String> {
    let payload = serde_json::to_value(credential)?;
    let key = store
        .public_key(key_id)
        .ok_or_else(|| anyhow::anyhow!("no key found for {key_id}"))?;
    let kid = format!("{}#{}", credential.issuer, key.thumbprint_uri()?);
    signer::sign_typed(store, key_id, &payload, &kid, Some(TYP), Some(CTY))
}

/// Whether a media-type header value names `expected`. RFC 7515 §4.1.9 lets a
/// producer omit the `application/` prefix and has a recipient read the value as
/// if it were there, and media types compare case-insensitively.
fn names_media_type(value: &str, expected: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.strip_prefix("application/").unwrap_or(&value) == expected
}

/// The refusal, if the protected header does not type the token as a VC-JWT:
/// `typ` must be present and name `vc+jwt`, and a `cty`, if present, must name
/// `vc`.
fn header_type_refusal(jws: &str) -> Option<String> {
    use base64::Engine as _;
    let Some(header) = jws
        .split('.')
        .next()
        .and_then(|h| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(h)
                .ok()
        })
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    else {
        return Some("the protected header is unreadable".to_owned());
    };
    let typ = header.get("typ");
    if !typ
        .and_then(serde_json::Value::as_str)
        .is_some_and(|t| names_media_type(t, TYP))
    {
        return Some(format!(
            "the token's typ is {}, not {TYP}",
            typ.map_or_else(|| "absent".to_owned(), ToString::to_string)
        ));
    }
    match header.get("cty") {
        None => None,
        Some(cty) if cty.as_str().is_some_and(|c| names_media_type(c, CTY)) => None,
        Some(cty) => Some(format!("the token's cty is {cty}, not {CTY}")),
    }
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
/// the document matches, the signature does not verify, the header does not type
/// the token as a VC-JWT, or the document belongs to a different issuer than the
/// credential claims.
/// [`VerificationResult::MalformedCredential`] when the payload is not a
/// credential.
pub fn authenticate_access_credential(
    jws: &str,
    issuer_did_document: &serde_json::Value,
) -> Result<DppAccessCredential, VerificationResult> {
    use base64::Engine as _;

    // The key the JWS names. A token that names none is refused, not verified
    // against a key chosen for it. The refusal says which document was checked
    // and what was looked for, since a document for another DID fails here.
    let key = resolve_verification_key(issuer_did_document, jws).ok_or_else(|| {
        let document_subject = issuer_did_document
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("(no id)");
        let kid = extract_kid_from_jws(jws).unwrap_or_else(|| "(none)".to_owned());
        VerificationResult::InvalidSignature(format!(
            "the DID document for {document_subject} publishes no key matching kid {kid}"
        ))
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

    // Read once the header is known to be the issuer's. A signature of any other
    // kind by the same key is authentic and still not a credential.
    if let Some(refusal) = header_type_refusal(jws) {
        return Err(VerificationResult::InvalidSignature(refusal));
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
