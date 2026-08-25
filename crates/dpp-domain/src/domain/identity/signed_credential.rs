//! [`SignedCredential`] — a passport credential with its JWS proof.

use serde::{Deserialize, Serialize};

use super::passport_credential::PassportCredential;

/// A DPP Verifiable Credential with its JWS proof signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedCredential {
    /// Structured W3C VC 2.0 passport credential.
    pub credential: PassportCredential,
    /// Compact JWS signature string (header.payload.signature).
    pub jws: String,
    /// The DID of the issuer (manufacturer or Odal on their behalf).
    pub issuer_did: String,
}
