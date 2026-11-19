//! The top-level members a credential or presentation may be signed with.
//!
//! The base vocabulary is the one the VC Data Model 2.0 context
//! (`https://www.w3.org/ns/credentials/v2`) defines: the terms its
//! `VerifiableCredential` and `VerifiablePresentation` type definitions scope to
//! each type, and the keywords `@context`, `id` and `type`. The two lists are
//! written out here, and a test in the `dpp-tests` crate derives them again from
//! the published context and compares, so they cannot drift from it.
//!
//! Signing refuses any other top-level member unless the caller declares it as an
//! extension term. Verification refuses none: VC-JOSE-COSE §3.1.3 and §5.4 say
//! members that are not understood MUST be ignored, so the refusal is an issuer's
//! discipline and never a verifier's.

use serde_json::{Map, Value};

use super::error::JoseError;

/// The top-level members of a Verifiable Credential in the VC Data Model 2.0
/// context: `@context`, then `id` and `type` (the context's aliases of `@id` and
/// `@type`), then every term its `VerifiableCredential` definition scopes.
///
/// `proof` is among them because the definition has it. A VC-JOSE-COSE credential
/// is secured by the JWS, not by an embedded proof, so a signer has no use for the
/// term and a verifier does not read it.
pub const CREDENTIAL_TERMS: [&str; 18] = [
    "@context",
    "id",
    "type",
    "confidenceMethod",
    "credentialSchema",
    "credentialStatus",
    "credentialSubject",
    "description",
    "evidence",
    "issuer",
    "name",
    "proof",
    "refreshService",
    "relatedResource",
    "renderMethod",
    "termsOfUse",
    "validFrom",
    "validUntil",
];

/// The top-level members of a Verifiable Presentation in the VC Data Model 2.0
/// context: `@context`, `id`, `type`, and every term its `VerifiablePresentation`
/// definition scopes.
///
/// `name` and `description` are not here. The context defines them for the whole
/// document and again inside `VerifiableCredential`, and not inside
/// `VerifiablePresentation`, which is the definition this list follows. A
/// presentation that carries them declares them as extension terms.
pub const PRESENTATION_TERMS: [&str; 7] = [
    "@context",
    "id",
    "type",
    "holder",
    "proof",
    "termsOfUse",
    "verifiableCredential",
];

/// The JWT registered claim names of RFC 7519 §4.1. VC-JOSE-COSE builds on that
/// RFC and speaks of `iss`, `iat` and `exp` itself (§3.1.3, §4.1.2), so a signer
/// accepts them without a declaration.
const REGISTERED_CLAIMS: [&str; 7] = ["iss", "sub", "aud", "exp", "nbf", "iat", "jti"];

/// The members of a document no claims set may carry (VC-JOSE-COSE §1.1.2.1).
const FORBIDDEN_CLAIMS: [&str; 2] = ["vc", "vp"];

/// Refuse a document a signer must not sign: one with a `vc` or `vp` member, or one
/// with a top-level member outside `base`, the JWT registered claims, and
/// `extension_terms`.
///
/// `vc` and `vp` are refused even when `extension_terms` names them.
pub(crate) fn check_members(
    document: &Map<String, Value>,
    base: &[&str],
    extension_terms: &[&str],
) -> Result<(), JoseError> {
    for name in FORBIDDEN_CLAIMS {
        if document.contains_key(name) {
            return Err(JoseError::ReservedClaim { name });
        }
    }
    let names: Vec<String> = document
        .keys()
        .filter(|name| {
            let name = name.as_str();
            !(base.contains(&name)
                || REGISTERED_CLAIMS.contains(&name)
                || extension_terms.contains(&name))
        })
        .cloned()
        .collect();
    if names.is_empty() {
        Ok(())
    } else {
        Err(JoseError::UndeclaredMembers { names })
    }
}
