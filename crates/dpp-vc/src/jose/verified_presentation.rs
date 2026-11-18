//! What a presentation hands back once its own token verifies.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;

use super::error::JoseError;
use super::unverified_envelope::{UnverifiedEnvelope, VC_JWT};

/// A presentation whose own token verified. Its signature and claims were checked; the
/// credentials it envelops were **not**.
///
/// VC-JOSE-COSE §5.1 defines verifying a presentation secured with JOSE as validating
/// its JWT, and §5 says key discovery is done for each document before verification
/// begins. So verifying the presentation and verifying each credential in it are two
/// operations. To use a credential, verify it: one at a time with
/// [`UnverifiedEnvelope::verify`], or every one of them with
/// [`verify_credentials`](Self::verify_credentials).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VerifiedPresentation {
    /// The presentation: the decoded payload of its JWS, with the envelopes as they
    /// arrived. Its signature and claims were verified.
    pub payload: Value,
    /// Every envelope of `verifiableCredential`, in order. Each was shape-checked and is
    /// NOT verified.
    pub envelopes: Vec<UnverifiedEnvelope>,
}

impl VerifiedPresentation {
    /// Verify every credential the presentation envelops, in order, each under the key
    /// `issuer_key` gives for it.
    ///
    /// `issuer_key` is asked once per envelope and answers `None` when the caller has no
    /// key for it. Every envelope must be a `vc+jwt`: an envelope of another format fails
    /// with [`JoseError::UnsupportedEnvelopeFormat`] before the key is asked for. The first
    /// envelope that fails fails the whole call, so a presentation with one bad credential
    /// is refused as a whole.
    ///
    /// # Errors
    ///
    /// [`JoseError::UnsupportedEnvelopeFormat`] for an envelope that is not a `vc+jwt`;
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
                if envelope.media_type != VC_JWT {
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
