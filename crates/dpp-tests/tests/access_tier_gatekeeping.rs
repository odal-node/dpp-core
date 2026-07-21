//! Integration test: Access tier gatekeeping.
//!
//! Exercises the full credential → policy → data-filtering pipeline
//! that protects DPP data behind the ESPR three-tier access model.
//!
//! 1. Build a textile DPP JSON payload with fields across all tiers.
//! 2. Create credentials for Public, Professional, and Confidential roles.
//! 3. Verify each credential and apply the SectorAccessPolicy.
//! 4. Assert that each tier sees only the fields it is allowed to see.
//! 5. Test edge cases: expired credentials, wrong-sector credentials,
//!    custom policies, and authority access.

use chrono::Utc;
use dpp_crypto::access::credential::{
    AccessTier, CredentialBuilder, CredentialRole, CredentialStatus, DppCredentialSubject,
    VerificationResult, verify_credential_claims,
};
use dpp_crypto::access::{SectorAccessPolicy, filter_by_access_tier};
use serde_json::json;

/// Build a realistic textile JSON payload with fields spanning all three tiers.
fn sample_textile_payload() -> serde_json::Value {
    json!({
        // ── Public tier fields ──
        "fibreComposition": [
            { "fibre": "cotton", "pct": 70.0 },
            { "fibre": "recycled_polyester", "pct": 30.0 }
        ],
        "countryOfOrigin": "BD",
        "careInstructions": "Machine wash 30°C",
        "carbonFootprintKgCo2e": 8.5,
        "durabilityScore": 7.5,
        "waterUseLitres": 2700.0,
        "recycledContentPct": 30.0,

        // ── Professional tier fields (repairer/recycler access) ──
        "svhcSubstances": [
            { "casNumber": "80-05-7", "substanceName": "Bisphenol A", "concentrationPct": 0.15 }
        ],
        "disassemblyInstructions": "Remove buttons, separate layers by colour group",
        "sparePartsAvailable": true,

        // ── Confidential tier fields (market surveillance) ──
        "jwsSignature": "eyJhbGciOiJFZERTQSJ9.payload.signature",
        "complianceReport": {
            "status": "compliant",
            "auditor": "TÜV SÜD",
            "auditDate": "2026-01-15"
        },
        "auditHistory": [
            { "date": "2025-12-01", "result": "pass" },
            { "date": "2026-01-15", "result": "pass" }
        ],
        "supplyChainTrace": {
            "rawMaterialSupplier": "IndoCotton Ltd",
            "spinningMill": "BanglaThread Co"
        }
    })
}

fn make_subject(
    did: &str,
    name: &str,
    role: CredentialRole,
    sectors: Vec<String>,
) -> DppCredentialSubject {
    DppCredentialSubject {
        id: did.into(),
        name: name.into(),
        role,
        country: "DE".into(),
        sectors,
        product_categories: vec![],
    }
}

// ─── Three-tier access tests ─────────────────────────────────────────────

#[test]
fn public_tier_sees_only_public_fields() {
    let data = sample_textile_payload();
    let policy = SectorAccessPolicy::from_catalog(&dpp_domain::SectorCatalog::new(), "textile")
        .expect("textile in catalog");

    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);

    // Public fields present
    assert!(decision.filtered_data.get("fibreComposition").is_some());
    assert!(decision.filtered_data.get("countryOfOrigin").is_some());
    assert!(decision.filtered_data.get("careInstructions").is_some());
    assert!(
        decision
            .filtered_data
            .get("carbonFootprintKgCo2e")
            .is_some()
    );
    assert!(decision.filtered_data.get("durabilityScore").is_some());

    // Professional fields redacted
    assert!(decision.filtered_data.get("svhcSubstances").is_none());
    assert!(
        decision
            .filtered_data
            .get("disassemblyInstructions")
            .is_none()
    );
    assert!(decision.filtered_data.get("sparePartsAvailable").is_none());

    // Confidential fields redacted
    assert!(decision.filtered_data.get("jwsSignature").is_none());
    assert!(decision.filtered_data.get("complianceReport").is_none());
    assert!(decision.filtered_data.get("auditHistory").is_none());
    assert!(decision.filtered_data.get("supplyChainTrace").is_none());

    // Redacted field list correct
    assert!(
        decision
            .redacted_fields
            .contains(&"svhcSubstances".to_string())
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"jwsSignature".to_string())
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"complianceReport".to_string())
    );
}

#[test]
fn professional_tier_via_repairer_credential() {
    let data = sample_textile_payload();
    let policy = SectorAccessPolicy::from_catalog(&dpp_domain::SectorCatalog::new(), "textile")
        .expect("textile in catalog");

    // Issue a credential to a repairer
    let subject = make_subject(
        "did:web:greenfix.example.com",
        "GreenFix Repair",
        CredentialRole::AuthorisedRepairer,
        vec!["textile".into()],
    );
    let credential = CredentialBuilder::new("did:web:textile-authority.eu".into(), subject)
        .expires_in_days(180)
        .build();

    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(result.is_valid());

    if let VerificationResult::Valid {
        access_tier,
        role,
        holder_did,
    } = &result
    {
        assert_eq!(*access_tier, AccessTier::Professional);
        assert_eq!(*role, CredentialRole::AuthorisedRepairer);
        assert_eq!(holder_did, "did:web:greenfix.example.com");

        let decision = filter_by_access_tier(&data, &policy, *access_tier);

        // Professional sees public + professional fields
        assert!(decision.filtered_data.get("fibreComposition").is_some());
        assert!(decision.filtered_data.get("svhcSubstances").is_some());
        assert!(
            decision
                .filtered_data
                .get("disassemblyInstructions")
                .is_some()
        );
        assert!(decision.filtered_data.get("sparePartsAvailable").is_some());

        // But NOT confidential
        assert!(decision.filtered_data.get("jwsSignature").is_none());
        assert!(decision.filtered_data.get("complianceReport").is_none());
        assert!(decision.filtered_data.get("auditHistory").is_none());
    }
}

#[test]
fn professional_tier_via_recycler_credential() {
    let subject = make_subject(
        "did:web:recyco.example.com",
        "RecyCo GmbH",
        CredentialRole::Recycler,
        vec!["textile".into()],
    );
    let credential = CredentialBuilder::new("did:web:textile-authority.eu".into(), subject)
        .expires_in_days(365)
        .build();

    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(result.is_valid());
    if let VerificationResult::Valid { access_tier, .. } = result {
        assert_eq!(access_tier, AccessTier::Professional);
    }
}

#[test]
fn confidential_tier_via_market_surveillance_authority() {
    let data = sample_textile_payload();
    let policy = SectorAccessPolicy::from_catalog(&dpp_domain::SectorCatalog::new(), "textile")
        .expect("textile in catalog");

    let subject = make_subject(
        "did:web:surveillance.europa.eu",
        "EU Market Surveillance Authority",
        CredentialRole::MarketSurveillanceAuthority,
        vec![], // empty = all sectors
    );
    let credential = CredentialBuilder::new("did:web:ec.europa.eu".into(), subject)
        .expires_in_days(365)
        .with_status(CredentialStatus {
            id: "https://ec.europa.eu/status/1#42".into(),
            status_type: "BitstringStatusListEntry".into(),
            status_list_index: Some("42".into()),
            status_list_credential: Some("https://ec.europa.eu/status/1".into()),
        })
        .build();

    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(result.is_valid());

    if let VerificationResult::Valid {
        access_tier, role, ..
    } = &result
    {
        assert_eq!(*access_tier, AccessTier::Confidential);
        assert_eq!(*role, CredentialRole::MarketSurveillanceAuthority);

        let decision = filter_by_access_tier(&data, &policy, *access_tier);

        // Confidential sees EVERYTHING
        assert!(decision.redacted_fields.is_empty());
        assert!(decision.filtered_data.get("fibreComposition").is_some());
        assert!(decision.filtered_data.get("svhcSubstances").is_some());
        assert!(decision.filtered_data.get("jwsSignature").is_some());
        assert!(decision.filtered_data.get("complianceReport").is_some());
        assert!(decision.filtered_data.get("auditHistory").is_some());
        assert!(decision.filtered_data.get("supplyChainTrace").is_some());
    }
}

#[test]
fn customs_authority_gets_confidential_access() {
    let subject = make_subject(
        "did:web:customs.de",
        "German Customs",
        CredentialRole::CustomsAuthority,
        vec![],
    );
    let credential = CredentialBuilder::new("did:web:bafin.de".into(), subject).build();

    let result = verify_credential_claims(&credential, None, Utc::now());
    if let VerificationResult::Valid { access_tier, .. } = result {
        assert_eq!(access_tier, AccessTier::Confidential);
    } else {
        panic!("customs authority credential should be valid");
    }
}

// ─── Edge cases ──────────────────────────────────────────────────────────

#[test]
fn expired_credential_denied() {
    let subject = make_subject(
        "did:web:expired.example.com",
        "Expired Co",
        CredentialRole::AuthorisedRepairer,
        vec!["textile".into()],
    );
    let mut credential =
        CredentialBuilder::new("did:web:authority.example.com".into(), subject).build();
    credential.valid_until = Utc::now() - chrono::Duration::hours(1);

    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(matches!(result, VerificationResult::Expired { .. }));
}

#[test]
fn wrong_sector_credential_rejected() {
    let subject = make_subject(
        "did:web:battery-recycler.example.com",
        "Battery Recycler",
        CredentialRole::Recycler,
        vec!["battery".into()],
    );
    let credential =
        CredentialBuilder::new("did:web:authority.example.com".into(), subject).build();

    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    assert!(
        matches!(result, VerificationResult::OutOfScope { .. }),
        "battery-scoped credential must not work for textile"
    );
}

#[test]
fn custom_policy_restricts_additional_fields() {
    let data = sample_textile_payload();
    let mut policy = SectorAccessPolicy::from_catalog(&dpp_domain::SectorCatalog::new(), "textile")
        .expect("textile in catalog");

    // Make durabilityScore professional-only (stricter than default)
    policy
        .field_tiers
        .insert("durabilityScore".into(), AccessTier::Professional);

    let public_decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert!(
        public_decision
            .filtered_data
            .get("durabilityScore")
            .is_none(),
        "custom policy should restrict durabilityScore at public tier"
    );
    assert!(
        public_decision
            .redacted_fields
            .contains(&"durabilityScore".to_string())
    );

    // Professional should still see it
    let pro_decision = filter_by_access_tier(&data, &policy, AccessTier::Professional);
    assert!(pro_decision.filtered_data.get("durabilityScore").is_some());
}

#[test]
fn all_credential_roles_map_to_correct_tiers() {
    // Professional roles
    let professional_roles = vec![
        CredentialRole::AuthorisedRepairer,
        CredentialRole::Recycler,
        CredentialRole::Remanufacturer,
        CredentialRole::PreparerForReuse,
        CredentialRole::Distributor,
        CredentialRole::Custom("textile_auditor".into()),
    ];
    for role in professional_roles {
        assert_eq!(
            role.access_tier(),
            AccessTier::Professional,
            "{role:?} should map to Professional tier"
        );
    }

    // Confidential roles
    let confidential_roles = vec![
        CredentialRole::MarketSurveillanceAuthority,
        CredentialRole::CustomsAuthority,
        CredentialRole::NotifiedBody,
    ];
    for role in confidential_roles {
        assert_eq!(
            role.access_tier(),
            AccessTier::Confidential,
            "{role:?} should map to Confidential tier"
        );
    }
}
