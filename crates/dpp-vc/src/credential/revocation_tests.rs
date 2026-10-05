//! `statusPurpose`: what a status entry says its list records, and why a
//! revocation check answers only for one.
//!
//! Bitstring Status List v1.0 requires the property as a string, and its test
//! suite checks that every entry has its own. Without it a verifier cannot tell a
//! revocation list from a suspension list, and a set bit means a different thing
//! in each.

use serde_json::json;

use crate::status_list::StatusList;

use super::*;

fn subject() -> DppCredentialSubject {
    DppCredentialSubject {
        id: "did:web:repairer.example".into(),
        name: "Repairs GmbH".into(),
        role: CredentialRole::AuthorisedRepairer,
        country: "DE".into(),
        product_groups: vec!["battery".into()],
        product_categories: Vec::new(),
    }
}

/// A credential whose status entry points at bit 0 of a list kept for `purpose`.
fn credential_for(purpose: &str) -> DppAccessCredential {
    CredentialBuilder::new("did:web:authority.example".into(), subject())
        .with_status(CredentialStatus {
            id: "https://authority.example/status/1#0".into(),
            status_type: "BitstringStatusListEntry".into(),
            status_purpose: purpose.into(),
            status_list_index: Some("0".into()),
            status_list_credential: Some("https://authority.example/status/1".into()),
        })
        .build()
}

/// Bit 0 set, and bit 0 clear.
fn set() -> StatusList {
    StatusList::from_bitstring(vec![0b1000_0000])
}
fn clear() -> StatusList {
    StatusList::from_bitstring(vec![0])
}

#[test]
fn an_entry_is_serialised_with_its_purpose() {
    let json = serde_json::to_value(credential_for(REVOCATION_PURPOSE)).expect("serialise");
    assert_eq!(json["credentialStatus"]["statusPurpose"], "revocation");
}

/// The specification and its test suite both require the property, so an entry
/// without one is not a status entry this crate reads.
#[test]
fn an_entry_without_a_purpose_does_not_deserialise() {
    let mut json = serde_json::to_value(credential_for(REVOCATION_PURPOSE)).expect("serialise");
    json["credentialStatus"]
        .as_object_mut()
        .expect("an entry")
        .remove("statusPurpose");

    assert!(serde_json::from_value::<DppAccessCredential>(json.clone()).is_err());

    // Control: the same document with the purpose put back reads.
    json["credentialStatus"]["statusPurpose"] = json!("revocation");
    assert!(serde_json::from_value::<DppAccessCredential>(json).is_ok());
}

/// The control: a revocation entry is answered by its bit.
#[test]
fn a_revocation_entry_is_answered_by_its_bit() {
    let credential = credential_for(REVOCATION_PURPOSE);
    assert_eq!(
        check_revocation(&credential, &set()),
        RevocationOutcome::Revoked
    );
    assert_eq!(
        check_revocation(&credential, &clear()),
        RevocationOutcome::NotRevoked
    );
}

/// A list kept for another purpose cannot answer a revocation check either way:
/// a set bit in a suspension list is not a revocation, and a clear one does not
/// say the credential is unrevoked.
#[test]
fn an_entry_for_another_purpose_is_indeterminate() {
    for purpose in ["suspension", "message", "Revocation", ""] {
        let credential = credential_for(purpose);
        for list in [set(), clear()] {
            assert_eq!(
                check_revocation(&credential, &list),
                RevocationOutcome::Indeterminate,
                "purpose {purpose:?}"
            );
        }
    }
}
