//! Building a passport's verifiable credential.

use dpp_domain::PassportId;
pub use dpp_domain::credential::{PassportCredential, PassportCredentialSubject};

/// The IRI that names a passport in linked data: `urn:uuid:` and its id.
///
/// One function, so the subject of a passport's credential and the `@id` of the
/// passport's own JSON-LD form ([`crate::frame_passport`]) are the same IRI. They
/// were not, and a graph holding both had two unrelated nodes for one passport.
/// `urn:uuid:` is the URN namespace for UUIDs that RFC 9562 keeps, and a passport
/// id is a UUID.
#[must_use]
pub fn passport_iri(passport_id: PassportId) -> String {
    format!("urn:uuid:{passport_id}")
}

/// Build the passport verifiable credential signed by `LocalIdentityService::sign_passport`.
pub(crate) fn build_passport_credential(
    issuer_did: String,
    passport_id: PassportId,
    payload_hash: String,
) -> PassportCredential {
    PassportCredential::new(
        issuer_did,
        PassportCredentialSubject {
            id: passport_iri(passport_id),
            payload_hash,
        },
    )
}
