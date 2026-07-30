//! Passport verifiable-credential construction.
//!
//! `PassportCredential`/`PassportCredentialSubject` are defined in `dpp-domain`
//! and re-exported here for callers that import from `dpp-crypto`. This module
//! additionally owns the signing-layer convention for turning a `PassportId`
//! into a VC subject id: `dpp-domain` stays URN-agnostic, so the `urn:uuid:`
//! formatting lives on the crypto side, next to the credential it's stamped into.

use dpp_domain::PassportId;
pub use dpp_domain::domain::identity::{PassportCredential, PassportCredentialSubject};

/// Build the passport verifiable credential signed by `LocalIdentityService::sign_passport`.
pub(crate) fn build_passport_credential(
    issuer_did: String,
    passport_id: PassportId,
    payload_hash: String,
) -> PassportCredential {
    PassportCredential::new(
        issuer_did,
        PassportCredentialSubject {
            id: format!("urn:uuid:{passport_id}"),
            payload_hash,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_id_is_the_passport_uuid_urn() {
        let passport_id = PassportId::new();
        let credential = build_passport_credential(
            "did:web:node.example.com".into(),
            passport_id,
            "deadbeef".into(),
        );
        assert_eq!(
            credential.credential_subject.id,
            format!("urn:uuid:{passport_id}")
        );
        assert_eq!(credential.issuer, "did:web:node.example.com");
    }
}
