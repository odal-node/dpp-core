//! One credential a presentation envelops: its envelope checked, and the credential not
//! verified.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;

use super::cose_verify::verify_credential_cose;
use super::data_url::{decode_data, decode_text};
use super::error::JoseError;
use super::options::{KeyBindingRequirement, VerifyOptions};
use super::sd_jwt_verify::verify_credential_sd_jwt;
use super::verify::verify_credential;

/// The media types of the credentials this crate verifies: one secured as a JWT, one
/// secured as an SD-JWT, and one secured with COSE.
pub(crate) const VC_JWT: &str = "application/vc+jwt";
pub(crate) const VC_SD_JWT: &str = "application/vc+sd-jwt";
pub(crate) const VC_COSE: &str = "application/vc+cose";

/// What follows the comma of an envelope's `data:` URL, still encoded, and whether the URL
/// says `;base64`. Nothing in it has been read.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct EnvelopeData {
    pub(crate) data: String,
    pub(crate) base64: bool,
}

/// An envelope of a presentation whose envelope was checked, and whose credential has not
/// been verified.
///
/// [`verify`](Self::verify) reads the content of the `data:` URL and verifies it under the
/// issuer's key. This type has no accessor that decodes the credential, so no claim of it is
/// returned unverified, and its data is private. The `kid` is read from the token's
/// protected header, which is not verified either: it is a hint for choosing a key, and
/// nothing more. The presentation that lists this envelope still carries the raw `data:`
/// URL in its payload, so the token is reachable through
/// [`VerifiedPresentation::payload`](super::VerifiedPresentation::payload).
#[derive(Clone, PartialEq, Eq)]
pub struct UnverifiedEnvelope {
    /// The envelope's position in `verifiableCredential`.
    pub index: usize,
    /// The media type its `id` names, lower-cased and without parameters:
    /// `application/vc+jwt`, `application/vc+sd-jwt`, `application/vc+cose` and so on.
    pub media_type: String,
    /// The `kid` the protected header of a `vc+jwt`, or of a `vc+sd-jwt`'s Issuer-signed JWT,
    /// names, or `None` when it names none or its data cannot be read as one, and always
    /// `None` for another media type. A `vc+cose`'s header is not read before its signature
    /// verifies, so its `kid` is not offered here.
    pub kid: Option<String>,
    /// The data of a `vc+jwt`, a `vc+sd-jwt` or a `vc+cose`; `None` for any other media type.
    data: Option<EnvelopeData>,
}

impl UnverifiedEnvelope {
    /// An envelope whose shape has been checked.
    pub(crate) fn new(
        index: usize,
        media_type: String,
        kid: Option<String>,
        data: Option<EnvelopeData>,
    ) -> Self {
        Self {
            index,
            media_type,
            kid,
            data,
        }
    }

    /// Verify the credential this envelope carries under `issuer_key`, with every check of
    /// [`verify_credential`](super::verify_credential),
    /// [`verify_credential_sd_jwt`](super::verify_credential_sd_jwt) for a `vc+sd-jwt`, or
    /// [`verify_credential_cose`](super::verify_credential_cose) for a `vc+cose`, and
    /// `options` as they take them. `binding` is the choice RFC 9901 §9.5 leaves to the
    /// verifier. A `vc+sd-jwt` is held to it. A `vc+jwt` and a `vc+cose` cannot carry a Key
    /// Binding JWT, so where binding is required they are refused, as RFC 9901 §7.3 step 2
    /// refuses an SD-JWT presented without one, and where it is not, they are verified as
    /// themselves.
    ///
    /// The data is read here and not before. A `vc+jwt` or `vc+sd-jwt` is the text the URL
    /// holds, percent-decoded and, for `;base64`, decoded as the WHATWG Infra Standard's
    /// "forgiving-base64 decode" does. A `vc+cose` is the bytes that decoding gives.
    ///
    /// # Errors
    ///
    /// [`JoseError::UnsupportedEnvelopeFormat`] when the envelope is none of `vc+jwt`,
    /// `vc+sd-jwt` and `vc+cose`, which are the formats this crate verifies.
    /// [`JoseError::CredentialRefused`] with [`JoseError::KeyBindingMissing`] for a `vc+jwt`
    /// or `vc+cose` when binding is required.
    /// [`JoseError::CredentialRefused`], naming this envelope and the check the credential
    /// failed, for one that does not verify, including data that is not readable as the
    /// format its media type names ([`JoseError::UnreadableEnvelopeData`],
    /// [`JoseError::NotCompactJws`], [`JoseError::MalformedSdJwt`] or [`JoseError::Cose`]).
    pub fn verify(
        &self,
        issuer_key: &PublicKey,
        options: VerifyOptions<'_>,
        binding: &KeyBindingRequirement,
    ) -> Result<Value, JoseError> {
        let Some(data) = self.data.as_ref() else {
            return Err(JoseError::UnsupportedEnvelopeFormat {
                index: self.index,
                media_type: self.media_type.clone(),
            });
        };
        let required = matches!(binding, KeyBindingRequirement::Required(_));
        let unreadable = |reason: &str| JoseError::UnreadableEnvelopeData {
            reason: reason.to_owned(),
        };
        let verified = match self.media_type.as_str() {
            VC_SD_JWT => decode_text(&data.data, data.base64)
                .map_err(unreadable)
                .and_then(|token| verify_credential_sd_jwt(&token, issuer_key, options, binding)),
            _ if required => Err(JoseError::KeyBindingMissing),
            VC_COSE => decode_data(&data.data, data.base64)
                .map_err(unreadable)
                .and_then(|bytes| verify_credential_cose(&bytes, issuer_key, options)),
            _ => decode_text(&data.data, data.base64)
                .map_err(unreadable)
                .and_then(|token| verify_credential(&token, issuer_key, options)),
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
