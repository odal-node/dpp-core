//! One credential a presentation envelops: shape-checked, and not verified.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;

use super::error::JoseError;
use super::verify::verify_credential;

/// The media type of a JWT-secured credential, the only one this crate verifies.
pub(crate) const VC_JWT: &str = "application/vc+jwt";

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
    /// The `kid` the protected header of a `vc+jwt` names, or `None` when it names none,
    /// and always `None` for another media type.
    pub kid: Option<String>,
    /// The compact JWS of a `vc+jwt`; `None` for any other media type.
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
    /// [`verify_credential`](super::verify_credential), and `now` as it takes it.
    ///
    /// # Errors
    ///
    /// [`JoseError::UnsupportedEnvelopeFormat`] when the envelope is not a `vc+jwt`, which
    /// this crate does not verify. [`JoseError::CredentialRefused`], naming this envelope
    /// and the check the credential failed, for a `vc+jwt` that does not verify.
    pub fn verify(&self, issuer_key: &PublicKey, now: Option<i64>) -> Result<Value, JoseError> {
        let Some(token) = self.token.as_deref() else {
            return Err(JoseError::UnsupportedEnvelopeFormat {
                index: self.index,
                media_type: self.media_type.clone(),
            });
        };
        verify_credential(token, issuer_key, now).map_err(|source| JoseError::CredentialRefused {
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
