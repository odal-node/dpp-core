//! Tests of the `passport_credential` module.

use super::super::*;

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
