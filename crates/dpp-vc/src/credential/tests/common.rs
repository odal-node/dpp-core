//! Credentials the tests share.

use super::super::*;

pub(super) fn sample_subject() -> DppCredentialSubject {
    DppCredentialSubject {
        id: "did:web:repair-shop.example.com".into(),
        name: "GreenFix Textile Repair".into(),
        role: CredentialRole::AuthorisedRepairer,
        country: "DE".into(),
        product_groups: vec!["textile".into()],
        product_categories: vec![],
    }
}

pub(super) fn sample_credential() -> DppAccessCredential {
    CredentialBuilder::new("did:web:authority.example.com".into(), sample_subject())
        .expires_in_days(365)
        .build()
}

pub(super) fn credential_with_status_index(index: &str) -> DppAccessCredential {
    CredentialBuilder::new("did:web:authority.example.com".into(), sample_subject())
        .with_status(CredentialStatus {
            id: format!("https://authority.example.com/status/1#{index}"),
            status_type: "BitstringStatusListEntry".into(),
            status_purpose: "revocation".into(),
            status_list_index: Some(index.to_owned()),
            status_list_credential: Some("https://authority.example.com/status/1".into()),
        })
        .build()
}
