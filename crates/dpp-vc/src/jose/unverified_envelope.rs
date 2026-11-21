//! One credential a presentation envelops: shape-checked, and not verified.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;

use super::error::JoseError;
use super::options::{KeyBindingRequirement, VerifyOptions};
use super::sd_jwt_verify::verify_credential_sd_jwt;
use super::verify::verify_credential;

/// The media types of the credentials this crate verifies: one secured as a JWT, and one
/// secured as an SD-JWT.
pub(crate) const VC_JWT: &str = "application/vc+jwt";
pub(crate) const VC_SD_JWT: &str = "application/vc+sd-jwt";

/// An envelope of a presentation whose shape was checked, and whose credential has not
/// been verified.
///
/// [`verify`](Self::verify) verifies it under the issuer's key. This type has no accessor
/// that decodes the credential, so no claim of it is returned unverified, and its compact
/// JWS is private. The `kid` is read from the token's protected header, which is not
/// verified either: it is a hint for choosing a key, and nothing more. The presentation
/// that lists this envelope still carries the raw `data:` URL in its payload, so the
/// token is reachable through [`VerifiedPresentation::payload`](super::VerifiedPresentation::payload).
#[derive(Clone, PartialEq, Eq)]
pub struct UnverifiedEnvelope {
    /// The envelope's position in `verifiableCredential`.
    pub index: usize,
    /// The media type its `id` names, lower-cased and without parameters:
    /// `application/vc+jwt`, `application/vc+sd-jwt`, `application/vc+cose` and so on.
    pub media_type: String,
    /// The `kid` the protected header of a `vc+jwt`, or of a `vc+sd-jwt`'s Issuer-signed JWT,
    /// names, or `None` when it names none,
    /// and always `None` for another media type.
    pub kid: Option<String>,
    /// The compact JWS of a `vc+jwt`, or the combined format of a `vc+sd-jwt`; `None` for any
    /// other media type.
    token: Option<String>,
}

impl UnverifiedEnvelope {
    /// An envelope whose shape has been checked.
    pub(crate) fn new(
        index: usize,
        media_type: String,
        kid: Option<String>,
        token: Option<String>,
    ) -> Self {
        Self {
            index,
            media_type,
            kid,
            token,
        }
    }

    /// Verify the credential this envelope carries under `issuer_key`, with every check of
    /// [`verify_credential`](super::verify_credential), or of
    /// [`verify_credential_sd_jwt`](super::verify_credential_sd_jwt) for a `vc+sd-jwt`, and
    /// `options` as they take them. `binding` is the choice RFC 9901 §9.5 leaves to the
    /// verifier. A `vc+sd-jwt` is held to it. A `vc+jwt` cannot carry a Key Binding JWT, so
    /// where binding is required it is refused, as RFC 9901 §7.3 step 2 refuses an SD-JWT
    /// presented without one, and where it is not, it is verified as a `vc+jwt`.
    ///
    /// # Errors
    ///
    /// [`JoseError::UnsupportedEnvelopeFormat`] when the envelope is neither a `vc+jwt` nor a
    /// `vc+sd-jwt`, which are the formats this crate verifies.
    /// [`JoseError::CredentialRefused`] with [`JoseError::KeyBindingMissing`] for a `vc+jwt`
    /// when binding is required.
    /// [`JoseError::CredentialRefused`], naming this envelope and the check the credential
    /// failed, for one that does not verify.
    pub fn verify(
        &self,
        issuer_key: &PublicKey,
        options: VerifyOptions<'_>,
        binding: &KeyBindingRequirement,
    ) -> Result<Value, JoseError> {
        let Some(token) = self.token.as_deref() else {
            return Err(JoseError::UnsupportedEnvelopeFormat {
                index: self.index,
                media_type: self.media_type.clone(),
            });
        };
        let verified = if self.media_type == VC_SD_JWT {
            verify_credential_sd_jwt(token, issuer_key, options, binding)
        } else if matches!(binding, KeyBindingRequirement::Required(_)) {
            Err(JoseError::KeyBindingMissing)
        } else {
            verify_credential(token, issuer_key, options)
        };
        verified.map_err(|source| JoseError::CredentialRefused {
            index: self.index,
            source: Box::new(source),
        })
    }
}

impl std::fmt::Debug for UnverifiedEnvelope {
    /// The token is left out of a `Debug` print, so the print shows the envelope's index,
    /// media type and `kid` and no part of the credential.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnverifiedEnvelope")
            .field("index", &self.index)
            .field("media_type", &self.media_type)
            .field("kid", &self.kid)
            .finish_non_exhaustive()
    }
}
