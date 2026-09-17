//! How issuing or verifying an SD-JWT VC fails.

use chrono::{DateTime, Utc};

use dpp_crypto::sd_jwt::SdJwtError;

use super::TYP;

/// Anything that can go wrong issuing or verifying an SD-JWT VC.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SdJwtVcError {
    /// The payload handed to [`super::issue`] was not a JSON object.
    #[error("payload is not a JSON object")]
    PayloadNotAnObject,
    /// The key store refused to sign.
    #[error("signing failed: {0}")]
    Signing(String),
    /// The Issuer-signed JWT's signature did not verify against the given key.
    #[error("issuer signature did not verify")]
    BadSignature,
    /// The disclosure mechanism rejected the token.
    #[error(transparent)]
    SdJwt(#[from] SdJwtError),
    /// `typ` is absent or is not [`TYP`].
    #[error("typ is '{0}', expected '{TYP}'")]
    WrongTyp(String),
    /// A claim clause 2.2.2 requires is missing.
    #[error("required claim '{0}' is absent")]
    MissingClaim(&'static str),
    /// A temporal claim is present but is not a JWT `NumericDate`.
    ///
    /// Refused rather than ignored: ignoring a malformed `exp` would read as an
    /// absent `exp`, which is the *unbounded* case.
    #[error("claim '{0}' is not a NumericDate")]
    MalformedTemporalClaim(&'static str),
    /// `exp` has passed.
    #[error("credential expired at {at}")]
    Expired {
        /// The instant the credential stopped being valid.
        at: DateTime<Utc>,
    },
    /// `nbf` has not been reached.
    #[error("credential is not valid until {from}")]
    NotYetValid {
        /// The instant the credential starts being valid.
        from: DateTime<Utc>,
    },
    /// The credential's `vct` is not the one the verifier expected.
    ///
    /// Checked because a verifier that accepts any `vct` is accepting a
    /// credential whose claim semantics it has not agreed to.
    #[error("vct is '{found}', expected '{expected}'")]
    UnexpectedVct {
        /// What the caller asked for.
        expected: String,
        /// What the credential declared.
        found: String,
    },
}
