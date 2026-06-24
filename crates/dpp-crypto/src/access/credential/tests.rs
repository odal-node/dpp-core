use chrono::{Duration, Utc};

use crate::access::status_list::StatusList;

use super::*;

fn sample_subject() -> DppCredentialSubject {
    DppCredentialSubject {
        id: "did:web:repair-shop.example.com".into(),
        name: "GreenFix Textile Repair".into(),
        role: CredentialRole::AuthorisedRepairer,
        country: "DE".into(),
        sectors: vec!["textile".into()],
        product_categories: vec![],
    }
}

fn sample_credential() -> DppAccessCredential {
    CredentialBuilder::new("did:web:authority.example.com".into(), sample_subject())
        .expires_in_days(365)
        .build()
}

#[test]
fn credential_builder_sets_required_fields() {
    let cred = sample_credential();
    assert!(
        cred.context
            .contains(&"https://www.w3.org/ns/credentials/v2".into())
    );
    assert!(
        cred.credential_type
            .contains(&"VerifiableCredential".into())
    );
    assert!(cred.credential_type.contains(&"DppAccessCredential".into()));
    assert!(cred.id.starts_with("urn:uuid:"));
    assert_eq!(cred.issuer, "did:web:authority.example.com");
    assert_eq!(
        cred.credential_subject.role,
        CredentialRole::AuthorisedRepairer
    );
}

#[test]
fn credential_round_trip() {
    let cred = sample_credential();
    let json = serde_json::to_value(&cred).unwrap();
    assert_eq!(json["@context"].as_array().unwrap().len(), 2);
    assert_eq!(json["credentialSubject"]["role"], "authorised_repairer");
    let back: DppAccessCredential = serde_json::from_value(json).unwrap();
    assert_eq!(cred.issuer, back.issuer);
    assert_eq!(cred.credential_subject.role, back.credential_subject.role);
}

#[test]
fn verify_valid_credential() {
    let cred = sample_credential();
    let result = verify_credential_claims(&cred, Some("textile"), Utc::now());
    assert!(result.is_valid());
    if let VerificationResult::Valid {
        access_tier, role, ..
    } = result
    {
        assert_eq!(access_tier, AccessTier::Professional);
        assert_eq!(role, CredentialRole::AuthorisedRepairer);
    }
}

#[test]
fn verify_expired_credential() {
    let mut cred = sample_credential();
    cred.valid_until = Utc::now() - Duration::hours(1);
    let result = verify_credential_claims(&cred, None, Utc::now());
    assert!(matches!(result, VerificationResult::Expired { .. }));
}

#[test]
fn verify_future_issuance_rejected() {
    let mut cred = sample_credential();
    cred.valid_from = Utc::now() + Duration::hours(1);
    let result = verify_credential_claims(&cred, None, Utc::now());
    assert!(matches!(result, VerificationResult::MalformedCredential(_)));
}

#[test]
fn verify_wrong_sector_out_of_scope() {
    let cred = sample_credential();
    let result = verify_credential_claims(&cred, Some("battery"), Utc::now());
    assert!(matches!(result, VerificationResult::OutOfScope { .. }));
}

#[test]
fn verify_empty_sectors_matches_all() {
    let mut subject = sample_subject();
    subject.sectors = vec![];
    let cred = CredentialBuilder::new("did:web:authority.example.com".into(), subject).build();
    let result = verify_credential_claims(&cred, Some("battery"), Utc::now());
    assert!(result.is_valid());
}

#[test]
fn authority_role_grants_confidential_tier() {
    let mut subject = sample_subject();
    subject.role = CredentialRole::MarketSurveillanceAuthority;
    subject.sectors = vec![];
    let cred = CredentialBuilder::new("did:web:surveillance.europa.eu".into(), subject).build();
    let result = verify_credential_claims(&cred, None, Utc::now());
    if let VerificationResult::Valid { access_tier, .. } = result {
        assert_eq!(access_tier, AccessTier::Confidential);
    } else {
        panic!("expected Valid result");
    }
}

#[test]
fn credential_with_status() {
    let cred = CredentialBuilder::new("did:web:authority.example.com".into(), sample_subject())
        .with_status(CredentialStatus {
            id: "https://authority.example.com/status/1#42".into(),
            status_type: "BitstringStatusListEntry".into(),
            status_list_index: Some("42".into()),
            status_list_credential: Some("https://authority.example.com/status/1".into()),
        })
        .build();

    let json = serde_json::to_value(&cred).unwrap();
    assert!(json["credentialStatus"].is_object());
    assert_eq!(json["credentialStatus"]["type"], "BitstringStatusListEntry");
}

/// VC-DM 2.0 conformance (crypto Gap 2).
#[test]
fn vc2_uses_valid_from_until_not_issuance_expiration() {
    let cred = CredentialBuilder::new("did:web:a.example.com".into(), sample_subject()).build();
    let json = serde_json::to_value(&cred).unwrap();
    assert_eq!(json["@context"][0], "https://www.w3.org/ns/credentials/v2");
    assert!(json.get("validFrom").is_some(), "VC 2.0 uses validFrom");
    assert!(json.get("validUntil").is_some(), "VC 2.0 uses validUntil");
    assert!(
        json.get("issuanceDate").is_none() && json.get("expirationDate").is_none(),
        "v1.1 property names must not appear under the v2.0 context"
    );
}

// ── Revocation (crypto Gap 5) ─────────────────────────────────────────────

fn credential_with_status_index(index: &str) -> DppAccessCredential {
    CredentialBuilder::new("did:web:authority.example.com".into(), sample_subject())
        .with_status(CredentialStatus {
            id: format!("https://authority.example.com/status/1#{index}"),
            status_type: "BitstringStatusListEntry".into(),
            status_list_index: Some(index.to_owned()),
            status_list_credential: Some("https://authority.example.com/status/1".into()),
        })
        .build()
}

#[test]
fn revoked_credential_is_detected() {
    let cred = credential_with_status_index("5");
    let list = StatusList::from_bitstring(vec![0b0000_0100]);
    assert_eq!(check_revocation(&cred, &list), RevocationOutcome::Revoked);
    assert_eq!(
        verify_credential_with_revocation(&cred, None, Utc::now(), Some(&list)),
        VerificationResult::Revoked
    );
}

#[test]
fn unrevoked_credential_passes() {
    let cred = credential_with_status_index("5");
    let list = StatusList::from_bitstring(vec![0b0000_0000]);
    assert_eq!(
        check_revocation(&cred, &list),
        RevocationOutcome::NotRevoked
    );
    assert!(verify_credential_with_revocation(&cred, None, Utc::now(), Some(&list)).is_valid());
}

#[test]
fn fail_closed_when_status_list_unavailable() {
    let cred = credential_with_status_index("5");
    assert_eq!(
        verify_credential_with_revocation(&cred, None, Utc::now(), None),
        VerificationResult::Revoked
    );
}

#[test]
fn no_status_means_revocation_not_applicable() {
    let cred = sample_credential();
    assert!(verify_credential_with_revocation(&cred, None, Utc::now(), None).is_valid());
}

#[test]
fn indeterminate_index_fails_closed() {
    let cred = credential_with_status_index("999");
    let list = StatusList::from_bitstring(vec![0u8]);
    assert_eq!(
        check_revocation(&cred, &list),
        RevocationOutcome::Indeterminate
    );
    assert_eq!(
        verify_credential_with_revocation(&cred, None, Utc::now(), Some(&list)),
        VerificationResult::Revoked
    );
}

#[test]
fn expired_credential_short_circuits_before_revocation() {
    let mut cred = credential_with_status_index("5");
    cred.valid_until = Utc::now() - Duration::hours(1);
    let list = StatusList::from_bitstring(vec![0b0000_0000]);
    assert!(matches!(
        verify_credential_with_revocation(&cred, None, Utc::now(), Some(&list)),
        VerificationResult::Expired { .. }
    ));
}

#[test]
fn credential_role_access_tiers() {
    assert_eq!(
        CredentialRole::AuthorisedRepairer.access_tier(),
        AccessTier::Professional
    );
    assert_eq!(
        CredentialRole::Recycler.access_tier(),
        AccessTier::Professional
    );
    assert_eq!(
        CredentialRole::MarketSurveillanceAuthority.access_tier(),
        AccessTier::Confidential
    );
    assert_eq!(
        CredentialRole::CustomsAuthority.access_tier(),
        AccessTier::Confidential
    );
    assert_eq!(
        CredentialRole::NotifiedBody.access_tier(),
        AccessTier::Confidential
    );
}

// ── Gap 9: issuer trust anchor ────────────────────────────────────────────

#[test]
fn trusted_issuer_passes() {
    let cred = sample_credential();
    let trusted = StaticTrustedIssuers::single(cred.issuer.clone());
    let result =
        verify_credential_claims_with_trust(&cred, Some("textile"), None, Utc::now(), &trusted);
    assert!(result.is_valid(), "trusted issuer must pass");
}

#[test]
fn untrusted_issuer_is_rejected() {
    let cred = sample_credential();
    let trusted = StaticTrustedIssuers::single("did:web:some-other-authority.example.com");
    let result =
        verify_credential_claims_with_trust(&cred, Some("textile"), None, Utc::now(), &trusted);
    assert!(
        matches!(result, VerificationResult::UntrustedIssuer { .. }),
        "unknown issuer must be rejected, got {result:?}"
    );
}

#[test]
fn allow_all_issuers_accepts_any_did() {
    let cred = sample_credential();
    let result = verify_credential_claims_with_trust(
        &cred,
        Some("textile"),
        None,
        Utc::now(),
        &AllowAllIssuers,
    );
    assert!(result.is_valid());
}

#[test]
fn professional_only_issuer_cannot_grant_confidential_tier() {
    let mut subject = sample_subject();
    subject.role = CredentialRole::MarketSurveillanceAuthority;
    subject.sectors = vec![];
    let issuer_did = "did:web:national-authority.example.com";
    let cred = CredentialBuilder::new(issuer_did.into(), subject).build();

    let trusted = StaticTrustedIssuers::new([issuer_did], std::iter::empty::<&str>());
    let result = verify_credential_claims_with_trust(&cred, None, None, Utc::now(), &trusted);
    assert!(
        matches!(result, VerificationResult::UntrustedIssuer { .. }),
        "professional-only issuer cannot grant Confidential tier"
    );
}

#[test]
fn confidential_issuer_can_also_grant_professional() {
    let cred = sample_credential();
    let trusted = StaticTrustedIssuers::new(std::iter::empty::<&str>(), [cred.issuer.as_str()]);
    let result =
        verify_credential_claims_with_trust(&cred, Some("textile"), None, Utc::now(), &trusted);
    assert!(
        result.is_valid(),
        "Confidential issuer can grant Professional"
    );
}

#[test]
fn product_category_scope_enforced() {
    let mut subject = sample_subject();
    subject.product_categories = vec!["smartphone".into()];
    let cred = CredentialBuilder::new("did:web:authority.example.com".into(), subject).build();
    let trusted = StaticTrustedIssuers::single("did:web:authority.example.com");

    let ok =
        verify_credential_claims_with_trust(&cred, None, Some("smartphone"), Utc::now(), &trusted);
    assert!(ok.is_valid(), "matching product category must pass");

    let bad = verify_credential_claims_with_trust(
        &cred,
        None,
        Some("washing-machine"),
        Utc::now(),
        &trusted,
    );
    assert!(
        matches!(bad, VerificationResult::OutOfScope { .. }),
        "mismatched product category must be OutOfScope"
    );
}

#[test]
fn empty_product_categories_covers_all() {
    let cred = sample_credential();
    let trusted = StaticTrustedIssuers::single(cred.issuer.clone());
    let result = verify_credential_claims_with_trust(
        &cred,
        None,
        Some("any-category"),
        Utc::now(),
        &trusted,
    );
    assert!(
        result.is_valid(),
        "empty product_categories = all categories"
    );
}

#[test]
fn trust_check_with_revocation_full_pipeline() {
    let cred = credential_with_status_index("5");
    let trusted = StaticTrustedIssuers::single(cred.issuer.clone());
    let list_clear = StatusList::from_bitstring(vec![0b0000_0000]);

    let result = verify_credential_with_revocation_and_trust(
        &cred,
        None,
        None,
        Utc::now(),
        Some(&list_clear),
        &trusted,
    );
    assert!(result.is_valid(), "unrevoked + trusted issuer must pass");

    let list_set = StatusList::from_bitstring(vec![0b0000_0100]);
    let revoked = verify_credential_with_revocation_and_trust(
        &cred,
        None,
        None,
        Utc::now(),
        Some(&list_set),
        &trusted,
    );
    assert_eq!(revoked, VerificationResult::Revoked);
}

// ── Remaining branch coverage ─────────────────────────────────────────────

#[test]
fn builder_expires_at_sets_exact_expiration() {
    let when = Utc::now() + Duration::days(30);
    let cred = CredentialBuilder::new("did:web:a.example.com".into(), sample_subject())
        .expires_at(when)
        .build();
    assert_eq!(cred.valid_until, when);
}

#[test]
fn missing_verifiable_credential_type_is_malformed() {
    let mut cred = sample_credential();
    cred.credential_type = vec!["DppAccessCredential".into()]; // no "VerifiableCredential"
    let result = verify_credential_claims(&cred, None, Utc::now());
    assert!(matches!(result, VerificationResult::MalformedCredential(_)));
}

#[test]
fn with_trust_short_circuits_on_invalid_base() {
    // An expired credential must surface Expired from the trust variant before
    // any issuer-trust check runs.
    let mut cred = sample_credential();
    cred.valid_until = Utc::now() - Duration::hours(1);
    let result =
        verify_credential_claims_with_trust(&cred, None, None, Utc::now(), &AllowAllIssuers);
    assert!(matches!(result, VerificationResult::Expired { .. }));
}

#[test]
fn revocation_and_trust_short_circuits_on_invalid_base() {
    let mut cred = credential_with_status_index("5");
    cred.valid_until = Utc::now() - Duration::hours(1);
    let trusted = StaticTrustedIssuers::single(cred.issuer.clone());
    let list = StatusList::from_bitstring(vec![0b0000_0000]);
    let result = verify_credential_with_revocation_and_trust(
        &cred,
        None,
        None,
        Utc::now(),
        Some(&list),
        &trusted,
    );
    assert!(matches!(result, VerificationResult::Expired { .. }));
}

#[test]
fn revocation_and_trust_passes_when_no_status_present() {
    // Trusted issuer, valid credential, no credentialStatus → nothing to revoke.
    let cred = sample_credential();
    let trusted = StaticTrustedIssuers::single(cred.issuer.clone());
    let result = verify_credential_with_revocation_and_trust(
        &cred,
        Some("textile"),
        None,
        Utc::now(),
        None,
        &trusted,
    );
    assert!(result.is_valid());
}

#[test]
fn revocation_and_trust_fails_closed_without_status_list() {
    // Has a credentialStatus but no list supplied → fail closed (Revoked).
    let cred = credential_with_status_index("5");
    let trusted = StaticTrustedIssuers::single(cred.issuer.clone());
    let result =
        verify_credential_with_revocation_and_trust(&cred, None, None, Utc::now(), None, &trusted);
    assert_eq!(result, VerificationResult::Revoked);
}

#[test]
fn check_revocation_without_status_is_not_revoked() {
    let cred = sample_credential(); // no credentialStatus
    let list = StatusList::from_bitstring(vec![0xFF]);
    assert_eq!(
        check_revocation(&cred, &list),
        RevocationOutcome::NotRevoked
    );
}

#[test]
fn check_revocation_unparseable_index_is_indeterminate() {
    let cred = credential_with_status_index("not-a-number");
    let list = StatusList::from_bitstring(vec![0xFF]);
    assert_eq!(
        check_revocation(&cred, &list),
        RevocationOutcome::Indeterminate
    );
}
