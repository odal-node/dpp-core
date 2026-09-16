//! How issuing or verifying an SD-JWT VC fails.

use chrono::{DateTime, Utc};

use dpp_crypto::sd_jwt::SdJwtError;

use super::TYP;

/// Anything that can go wrong issuing or verifying an SD-JWT VC.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SdJwtVcError {
    /// The payload handed to [`super::issue`] was not a JSON object.
    PayloadNotAnObject,
    /// The key store refused to sign.
    Signing(String),
    /// The Issuer-signed JWT's signature did not verify against the given key.
    BadSignature,
    /// The disclosure mechanism rejected the token.
    SdJwt(SdJwtError),
    /// `typ` is absent or is not [`TYP`].
    WrongTyp(String),
    /// A claim clause 2.2.2 requires is missing.
    MissingClaim(&'static str),
    /// A temporal claim is present but is not a JWT `NumericDate`.
    ///
    /// Refused rather than ignored: ignoring a malformed `exp` would read as an
    /// absent `exp`, which is the *unbounded* case.
    MalformedTemporalClaim(&'static str),
    /// `exp` has passed.
    Expired {
        /// The instant the credential stopped being valid.
        at: DateTime<Utc>,
    },
    /// `nbf` has not been reached.
    NotYetValid {
        /// The instant the credential starts being valid.
        from: DateTime<Utc>,
    },
    /// The credential's `vct` is not the one the verifier expected.
    ///
    /// Checked because a verifier that accepts any `vct` is accepting a
    /// credential whose claim semantics it has not agreed to.
    UnexpectedVct {
        /// What the caller asked for.
        expected: String,
        /// What the credential declared.
        found: String,
    },
}

impl std::fmt::Display for SdJwtVcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PayloadNotAnObject => f.write_str("payload is not a JSON object"),
            Self::Signing(e) => write!(f, "signing failed: {e}"),
            Self::BadSignature => f.write_str("issuer signature did not verify"),
            Self::SdJwt(e) => write!(f, "{e}"),
            Self::WrongTyp(found) => write!(f, "typ is '{found}', expected '{TYP}'"),
            Self::MissingClaim(c) => write!(f, "required claim '{c}' is absent"),
            Self::MalformedTemporalClaim(c) => {
                write!(f, "claim '{c}' is not a NumericDate")
            }
            Self::Expired { at } => write!(f, "credential expired at {at}"),
            Self::NotYetValid { from } => write!(f, "credential is not valid until {from}"),
            Self::UnexpectedVct { expected, found } => {
                write!(f, "vct is '{found}', expected '{expected}'")
            }
        }
    }
}

impl std::error::Error for SdJwtVcError {}

impl From<SdJwtError> for SdJwtVcError {
    fn from(e: SdJwtError) -> Self {
        Self::SdJwt(e)
    }
}
