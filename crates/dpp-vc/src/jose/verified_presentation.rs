//! What a presentation hands back once its own token verifies.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;

use super::error::JoseError;
use super::unverified_envelope::{UnverifiedEnvelope, VC_JWT, VC_SD_JWT};

/// A presentation whose own token verified. Its signature and claims were checked; the
/// credentials it envelops were **not**.
///
/// VC-JOSE-COSE §5.1 defines verifying a presentation secured with JOSE as validating
/// its JWT, and §5 says key discovery is done for each document before verification
/// begins. So verifying the presentation and verifying each credential in it are two
/// operations. To use a credential, verify it: one at a time with
/// [`UnverifiedEnvelope::verify`], or every one of them with
/// [`verify_credentials`](Self::verify_credentials).
///
/// The fields are private, so the envelopes cannot be dropped or reordered between
/// `verify_presentation` and `verify_credentials`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VerifiedPresentation {
    payload: Value,
    envelopes: Vec<UnverifiedEnvelope>,
}

impl VerifiedPresentation {
    /// A presentation whose own token verified, with its envelopes listed.
    pub(crate) fn new(payload: Value, envelopes: Vec<UnverifiedEnvelope>) -> Self {
        Self { payload, envelopes }
    }

    /// The presentation: the decoded payload of its JWS. Its signature and claims were
    /// verified.
    ///
    /// The payload still carries each envelope's raw `data:` URL, unverified: the compact
    /// JWS of a `vc+jwt`, or the SD-JWT of a `vc+sd-jwt`, is in it as text. Reading the payload therefore gives the
    /// credentials as they arrived, not their verified claims. A credential's verified
    /// claims are what [`verify_credentials`](Self::verify_credentials) or
    /// [`UnverifiedEnvelope::verify`] return.
    pub fn payload(&self) -> &Value {
        &self.payload
    }

    /// Every envelope of `verifiableCredential`, in order. Each was shape-checked and is
    /// NOT verified.
    pub fn envelopes(&self) -> &[UnverifiedEnvelope] {
        &self.envelopes
    }

    /// Verify every credential the presentation envelops, in order, each under the key
    /// `issuer_key` gives for it.
    ///
    /// `issuer_key` is asked once per envelope and answers `None` when the caller has no
    /// key for it. Every envelope must be a `vc+jwt` or a `vc+sd-jwt`: one of another format fails
    /// with [`JoseError::UnsupportedEnvelopeFormat`] before the key is asked for. The first
    /// envelope that fails fails the whole call, so a presentation with one bad credential
    /// is refused as a whole.
    ///
    /// # Errors
    ///
    /// [`JoseError::UnsupportedEnvelopeFormat`] for an envelope that is neither a `vc+jwt` nor a `vc+sd-jwt`;
    /// [`JoseError::CredentialKeyUnknown`] for an envelope with no key;
    /// [`JoseError::CredentialRefused`] for a credential that fails a check.
    pub fn verify_credentials<F>(
        &self,
        now: Option<i64>,
        mut issuer_key: F,
    ) -> Result<Vec<Value>, JoseError>
    where
        F: FnMut(&UnverifiedEnvelope) -> Option<PublicKey>,
    {
        self.envelopes
            .iter()
            .map(|envelope| {
                if envelope.media_type != VC_JWT && envelope.media_type != VC_SD_JWT {
                    return Err(JoseError::UnsupportedEnvelopeFormat {
                        index: envelope.index,
                        media_type: envelope.media_type.clone(),
                    });
                }
                let key = issuer_key(envelope).ok_or_else(|| JoseError::CredentialKeyUnknown {
                    index: envelope.index,
                    kid: envelope.kid.clone(),
                })?;
                envelope.verify(&key, now)
            })
            .collect()
    }
}
