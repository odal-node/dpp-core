//! Signing a credential or a presentation as a VC-JOSE-COSE SD-JWT.

use serde_json::{Map, Value};

use dpp_crypto::jws::PublicKey;
use dpp_crypto::jws::signer::sign_typed_with;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::{ClaimPath, PathSegment, SdJwt, build_payload, holder_key_confirmation};

use super::disclose::conceal_paths;
use super::error::JoseError;
use super::properties::{
    check_context, check_credential_properties, check_holder, check_presentation_properties,
};
use super::sd_jwt_verify::{CREDENTIAL_SD_JWT_TYP, PRESENTATION_SD_JWT_TYP};
use super::sign::object;
use super::verify::{
    CREDENTIAL_CTY, PRESENTATION_CTY, check_holder_iss, check_holder_kid, check_issuer, check_kid,
    check_temporal_claims,
};
use super::vocabulary::{CREDENTIAL_TERMS, PRESENTATION_TERMS, check_members};

/// The claims VC-JOSE-COSE §3.2.1 and §3.2.2 say "SHOULD be disclosed" because they are
/// "necessary for the validation and verification of a credential": the specification
/// lists `@context`, `type`, `credentialStatus`, `credentialSchema` and `relatedResource`.
/// A signer here takes the SHOULD as a rule.
const NECESSARY: [&str; 5] = [
    "@context",
    "type",
    "credentialStatus",
    "credentialSchema",
    "relatedResource",
];

/// The claims RFC 9901 clause 9.7 says an Issuer must not make selectively disclosable:
/// "critical for evaluating the SD-JWT's authenticity or validity". `iss`, `aud`, `exp`,
/// `nbf` and `cnf` are the ones it names. A signer here refuses a path that starts at one.
/// A verifier does not read the way a claim travelled: it checks the claim wherever it is in
/// the processed payload.
const NEVER_DISCLOSED: [&str; 5] = ["iss", "aud", "exp", "nbf", "cnf"];

/// Sign a credential as a `vc+sd-jwt` (VC-JOSE-COSE §3.2.1), concealing the claims
/// `disclose` names, and return the SD-JWT with every Disclosure it makes.
///
/// The credential is the input JWT Claims Set, and the SD-JWT payload is made from it as
/// RFC 9901 clause 4 says: each claim a path names is replaced by a digest, and `_sd_alg`
/// is added when anything is concealed. Serialise the result with
/// [`SdJwt::serialise`], or choose what to present with [`SdJwt::present`]. The protected
/// header carries the key's `alg`, the `kid` given, `typ: vc+sd-jwt` and `cty: vc`.
///
/// No claim is added and nothing is stamped, except `cnf` when `holder_key` is given, and
/// the Disclosures carry random salts, so two calls give two tokens.
///
/// # Key Binding
///
/// With a `holder_key`, the payload carries `cnf` naming it, as
/// [`holder_key_confirmation`] writes it: RFC 7800 §3.2's `jwk`, which RFC 9901 clause 4.1.2
/// suggests and VC-JOSE-COSE §4.1.3 lets a verifier use. The holder then binds a
/// presentation with
/// [`SdJwt::present_with_key_binding`], and a verifier that requires Key Binding checks it
/// with [`KeyBindingRequirement::Required`](super::KeyBindingRequirement::Required). `cnf` is
/// written in the clear and cannot be concealed. A document that already carries `cnf` is
/// refused when a key is given ([`JoseError::ConfirmationClaimPresent`]), and not overwritten.
///
/// # Refused before signing
///
/// Everything [`sign_credential`](super::sign_credential) refuses, on the credential as
/// given, and:
///
/// - A path whose first member is `@context`, `type`, `credentialStatus`,
///   `credentialSchema` or `relatedResource`, which VC-JOSE-COSE §3.2.1 says "SHOULD be
///   disclosed", or `iss`, `aud`, `exp`, `nbf` or `cnf`, which RFC 9901 clause 9.7 says an
///   Issuer must not make selectively disclosable ([`JoseError::UnconcealableClaim`]).
/// - A path that selects nothing ([`JoseError::DisclosurePathNotFound`]).
/// - A member named `_sd`, `_sd_alg` or `...` anywhere ([`JoseError::ReservedSdJwtMember`]).
///
/// # Errors
///
/// A [`JoseError`] naming what was refused, or [`JoseError::Signing`] when the signer
/// failed.
pub fn sign_credential_sd_jwt(
    credential: &Value,
    key: &KeyEntry,
    kid: &str,
    extension_terms: &[&str],
    disclose: &[ClaimPath],
    holder_key: Option<&PublicKey>,
) -> Result<SdJwt, JoseError> {
    let document = object(credential, "credential")?;
    check_members(document, &CREDENTIAL_TERMS, extension_terms)?;
    check_context(document)?;
    check_temporal_claims(document)?;
    check_issuer(document)?;
    check_kid(document, kid)?;
    check_credential_properties(document)?;
    seal(
        document,
        key,
        kid,
        CREDENTIAL_SD_JWT_TYP,
        CREDENTIAL_CTY,
        disclose,
        holder_key,
    )
}

/// Sign a presentation as a `vp+sd-jwt` (VC-JOSE-COSE §3.2.2), concealing the claims
/// `disclose` names, and return the SD-JWT with every Disclosure it makes.
///
/// The members are checked as [`sign_credential_sd_jwt`] checks them, against
/// [`PRESENTATION_TERMS`], and the paths are held to the same rules. The credentials a
/// presentation holds are given already enveloped, and are not opened. A path may name
/// `holder`, or one envelope as `["verifiableCredential", 0]`.
///
/// The protected header carries `typ: vp+sd-jwt` and `cty: vp`. `holder_key` is as for
/// [`sign_credential_sd_jwt`]: the key a verifier that requires Key Binding checks the
/// presentation against.
///
/// # Errors
///
/// A [`JoseError`] naming what was refused, or [`JoseError::Signing`] when the signer
/// failed.
pub fn sign_presentation_sd_jwt(
    presentation: &Value,
    key: &KeyEntry,
    kid: &str,
    extension_terms: &[&str],
    disclose: &[ClaimPath],
    holder_key: Option<&PublicKey>,
) -> Result<SdJwt, JoseError> {
    let document = object(presentation, "presentation")?;
    check_members(document, &PRESENTATION_TERMS, extension_terms)?;
    check_context(document)?;
    check_temporal_claims(document)?;
    check_holder(document)?;
    check_holder_iss(document)?;
    check_holder_kid(document, kid)?;
    check_presentation_properties(document)?;
    seal(
        document,
        key,
        kid,
        PRESENTATION_SD_JWT_TYP,
        PRESENTATION_CTY,
        disclose,
        holder_key,
    )
}

fn seal(
    document: &Map<String, Value>,
    key: &KeyEntry,
    kid: &str,
    typ: &str,
    cty: &str,
    disclose: &[ClaimPath],
    holder_key: Option<&PublicKey>,
) -> Result<SdJwt, JoseError> {
    for path in disclose {
        if let Some(PathSegment::Key(name)) = path.segments().first()
            && (NECESSARY.contains(&name.as_str()) || NEVER_DISCLOSED.contains(&name.as_str()))
        {
            return Err(JoseError::UnconcealableClaim { name: name.clone() });
        }
    }
    let (concealed, disclosures) = match holder_key {
        None => conceal_paths(document, disclose)?,
        Some(holder_key) => {
            if document.contains_key("cnf") {
                return Err(JoseError::ConfirmationClaimPresent);
            }
            let mut bound = document.clone();
            bound.insert("cnf".to_owned(), holder_key_confirmation(holder_key));
            conceal_paths(&bound, disclose)?
        }
    };
    let payload = build_payload(concealed, !disclosures.is_empty());
    let jwt = sign_typed_with(key, &payload, kid, Some(typ), Some(cty)).map_err(|e| {
        JoseError::Signing {
            reason: e.to_string(),
        }
    })?;
    Ok(SdJwt::new(jwt, disclosures))
}
