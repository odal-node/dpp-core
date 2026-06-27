//! Integration tests: adversarial security scenarios (Phase 3.0a).
//!
//! Each test targets a specific Phase-1 security fix, turning the audit
//! finding into a cross-crate verifiable assertion. Scenarios covered:
//!
//! - **Revocation denied (fail-closed)**: credential with status bit SET → Revoked;
//!   status list unavailable → Revoked (never fail-open).
//! - **Forged-issuer rejected**: `StaticTrustedIssuers` blocks untrusted issuer DID.
//! - **Content-binding tamper rejected**: JWS signed over canonical payload A cannot
//!   be presented with payload B — swapping the payload segment fails verification.
//! - **Fail-closed redaction**: `default_tier = Confidential` gates unlisted fields.
//! - **Transfer deadlock resolved**: `reject()` / `cancel()` make terminal states
//!   reachable, unblocking the chain for a new transfer.
//! - **Passport sector-data validation**: `Passport::validate()` now wires into
//!   `validate_sector_data()`, catching cross-field errors (e.g. fibre sum ≠ 100).

use chrono::Utc;
use dpp_crypto::access::{AccessTier, SectorAccessPolicy, filter_by_access_tier};
use dpp_crypto::{
    AllowAllIssuers, CredentialBuilder, CredentialRole, CredentialStatus, DppCredentialSubject,
    StaticTrustedIssuers, StatusList, VerificationResult, verify_credential_claims_with_trust,
    verify_credential_with_revocation,
};
use dpp_domain::{
    FibreEntry, ManufacturerInfo, Passport, PassportId, PassportStatus, Sector, SectorData,
    TextileData, TransferChain, TransferError, TransferReason, TransferRecord, TransferStatus,
};
use serde_json::json;
use uuid::Uuid;

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn make_subject(role: CredentialRole, sectors: Vec<String>) -> DppCredentialSubject {
    DppCredentialSubject {
        id: "did:web:holder.example.com".into(),
        name: "Test Holder".into(),
        role,
        country: "DE".into(),
        sectors,
        product_categories: vec![],
    }
}

/// A StatusList where bit 5 is SET (revoked).
fn status_list_with_bit_5_set() -> StatusList {
    // Big-endian within byte: bit 5 = 0x80 >> 5 = 0b0000_0100.
    StatusList::from_bitstring(vec![0b0000_0100, 0b0000_0000])
}

/// A StatusList where all bits are CLEAR.
fn status_list_all_clear() -> StatusList {
    StatusList::from_bitstring(vec![0b0000_0000, 0b0000_0000])
}

fn credential_with_status_at_index(index: &str) -> dpp_crypto::DppAccessCredential {
    let subject = make_subject(CredentialRole::AuthorisedRepairer, vec!["textile".into()]);
    CredentialBuilder::new("did:web:authority.example.com".into(), subject)
        .with_status(CredentialStatus {
            id: format!("https://status.example.com/list#{index}"),
            status_type: "BitstringStatusListEntry".into(),
            status_list_index: Some(index.into()),
            status_list_credential: Some("https://status.example.com/list".into()),
        })
        .expires_in_days(365)
        .build()
}

fn temp_key_store() -> dpp_crypto::keystore::KeyStore {
    let path = std::env::temp_dir().join(format!("adv-test-{}.json", Uuid::now_v7()));
    let store = dpp_crypto::keystore::KeyStore::open(path, "test").expect("open store");
    store.generate_key("key").expect("generate key");
    store
}

// ─── Revocation tests ─────────────────────────────────────────────────────────

/// A credential whose status list bit is SET must be rejected as Revoked.
#[test]
fn revoked_credential_denied() {
    let credential = credential_with_status_at_index("5");
    let list = status_list_with_bit_5_set();

    let result =
        verify_credential_with_revocation(&credential, Some("textile"), Utc::now(), Some(&list));

    assert_eq!(
        result,
        VerificationResult::Revoked,
        "credential with revocation bit set must be denied"
    );
}

/// A credential whose status list cannot be fetched (None) must be rejected.
/// This is the fail-closed property: if revocation status is unresolvable,
/// the credential MUST NOT grant access.
#[test]
fn revoked_credential_fail_closed_when_list_unavailable() {
    let credential = credential_with_status_at_index("5");

    // `None` simulates a status-list fetch failure (network error, timeout, etc.).
    let result = verify_credential_with_revocation(&credential, Some("textile"), Utc::now(), None);

    assert_eq!(
        result,
        VerificationResult::Revoked,
        "unavailable status list must fail closed (Revoked), not grant access"
    );
}

/// A credential whose status list bit is CLEAR must pass revocation check.
#[test]
fn non_revoked_credential_passes_revocation_check() {
    let credential = credential_with_status_at_index("5");
    let list = status_list_all_clear(); // bit 5 is clear

    let result =
        verify_credential_with_revocation(&credential, Some("textile"), Utc::now(), Some(&list));

    assert!(
        result.is_valid(),
        "credential with clear revocation bit must pass, got {result:?}"
    );
}

/// A credential with no credentialStatus declared is not subject to revocation.
#[test]
fn credential_without_status_declaration_unaffected_by_revocation_check() {
    let subject = make_subject(CredentialRole::AuthorisedRepairer, vec!["textile".into()]);
    let credential = CredentialBuilder::new("did:web:authority.example.com".into(), subject)
        .expires_in_days(365)
        .build();

    // Passing None for the list is safe when no status is declared.
    let result = verify_credential_with_revocation(&credential, Some("textile"), Utc::now(), None);

    assert!(
        result.is_valid(),
        "credential without credentialStatus is not affected by revocation check"
    );
}

// ─── Forged-issuer tests ───────────────────────────────────────────────────────

/// A credential from an untrusted issuer DID must be rejected even when the
/// claims are otherwise structurally valid.
#[test]
fn forged_issuer_credential_rejected() {
    let subject = make_subject(CredentialRole::AuthorisedRepairer, vec!["textile".into()]);
    // Issued by an attacker DID — not in the operator's trust registry.
    let credential = CredentialBuilder::new("did:web:evil-attacker.example.com".into(), subject)
        .expires_in_days(365)
        .build();

    let trusted = StaticTrustedIssuers::single("did:web:legit-authority.eu");

    let result = verify_credential_claims_with_trust(
        &credential,
        Some("textile"),
        None,
        Utc::now(),
        &trusted,
    );

    assert!(
        matches!(result, VerificationResult::UntrustedIssuer { ref issuer_did } if issuer_did.contains("evil")),
        "credential from untrusted issuer must be rejected, got {result:?}"
    );
}

/// A credential from the registered trusted issuer must pass the trust check.
#[test]
fn trusted_issuer_credential_passes() {
    let subject = make_subject(CredentialRole::AuthorisedRepairer, vec!["textile".into()]);
    let trusted_did = "did:web:legit-authority.eu";
    let credential = CredentialBuilder::new(trusted_did.into(), subject)
        .expires_in_days(365)
        .build();

    let trusted = StaticTrustedIssuers::single(trusted_did);

    let result = verify_credential_claims_with_trust(
        &credential,
        Some("textile"),
        None,
        Utc::now(),
        &trusted,
    );

    assert!(
        result.is_valid(),
        "credential from trusted issuer must pass, got {result:?}"
    );
}

/// AllowAllIssuers is only safe in tests; confirms the bypass semantics.
#[test]
fn allow_all_issuers_accepts_any_did() {
    let subject = make_subject(CredentialRole::Recycler, vec![]);
    let credential = CredentialBuilder::new("did:web:whoever.example.com".into(), subject).build();

    let result =
        verify_credential_claims_with_trust(&credential, None, None, Utc::now(), &AllowAllIssuers);

    assert!(
        result.is_valid(),
        "AllowAllIssuers must accept any issuer (test bypass)"
    );
}

// ─── Content-binding tamper tests ─────────────────────────────────────────────

/// The JWS is signed over the RFC 8785 canonical form of the payload. Swapping
/// the payload segment of a JWS while keeping the original signature must fail
/// verification — this is the content-binding contract.
#[test]
fn content_binding_tamper_rejected() {
    let store = temp_key_store();

    let original = json!({"passportId": "abc-123", "status": "active"});
    let tampered = json!({"passportId": "abc-123", "status": "suspended"});

    // Sign original payload
    let jws_original =
        dpp_crypto::jws::signer::sign(&store, "key", &original).expect("sign original");
    // Sign tampered payload (used only to get the canonical tampered payload_b64)
    let jws_tampered_payload =
        dpp_crypto::jws::signer::sign(&store, "key", &tampered).expect("sign tampered");

    // Build forged JWS: original header + sig, but tampered payload segment.
    let parts_orig: Vec<&str> = jws_original.splitn(3, '.').collect();
    let parts_tamp: Vec<&str> = jws_tampered_payload.splitn(3, '.').collect();
    assert_eq!(parts_orig.len(), 3);
    assert_eq!(parts_tamp.len(), 3);

    let forged_jws = format!("{}.{}.{}", parts_orig[0], parts_tamp[1], parts_orig[2]);

    let ok = dpp_crypto::jws::signer::verify(&store, "key", &forged_jws)
        .expect("verify must not error on malformed JWS");
    assert!(
        !ok,
        "swapping the payload segment while keeping the original signature must fail"
    );

    // Sanity: original JWS verifies correctly.
    let ok_orig =
        dpp_crypto::jws::signer::verify(&store, "key", &jws_original).expect("verify original");
    assert!(ok_orig, "original JWS must verify");
}

/// A JWS with the signature byte-flipped is rejected.
#[test]
fn signature_tamper_rejected() {
    let store = temp_key_store();
    let payload = json!({"passportId": "xyz-456", "status": "active"});
    let mut jws = dpp_crypto::jws::signer::sign(&store, "key", &payload).expect("sign");

    // Flip the last character of the signature.
    let last = jws.pop().unwrap();
    jws.push(if last == 'A' { 'B' } else { 'A' });

    let ok = dpp_crypto::jws::signer::verify(&store, "key", &jws).unwrap_or(false);
    assert!(!ok, "signature tamper must be rejected");
}

// ─── Fail-closed redaction tests ──────────────────────────────────────────────

/// With `default_tier = Confidential`, fields absent from the policy are
/// redacted at the Public tier — no unlisted field leaks.
#[test]
fn fail_closed_default_tier_blocks_unlisted_fields() {
    use std::collections::HashMap;

    let policy = SectorAccessPolicy {
        name: "strict-policy".into(),
        sector: "test".into(),
        field_tiers: {
            let mut m = HashMap::new();
            m.insert("productName".into(), AccessTier::Public);
            m.insert("co2ePerUnit".into(), AccessTier::Public);
            m
        },
        default_tier: AccessTier::Confidential, // fail-closed: unlisted = Confidential
    };

    let data = json!({
        "productName": "EcoWidget",
        "co2ePerUnit": 5.2,
        "internalAuditScore": 9.5,   // unlisted — must be redacted
        "supplyChainSecret": "...",  // unlisted — must be redacted
    });

    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);

    assert_eq!(decision.filtered_data["productName"], json!("EcoWidget"));
    assert!(
        decision.filtered_data.get("internalAuditScore").is_none(),
        "unlisted field must be redacted in fail-closed mode"
    );
    assert!(
        decision.filtered_data.get("supplyChainSecret").is_none(),
        "unlisted field must be redacted in fail-closed mode"
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"internalAuditScore".to_owned())
    );
}

/// Nested confidential fields cannot bypass redaction via nesting.
#[test]
fn nested_confidential_field_cannot_bypass_via_nesting() {
    let policy = SectorAccessPolicy::from_catalog(&dpp_domain::SectorCatalog::new(), "textile")
        .expect("textile in catalog");

    let data = json!({
        "fibreComposition": [{"fibre": "cotton", "pct": 100.0}],
        "countryOfManufacturing": "DE",
        "sectorData": {
            "careInstructions": "cold wash",
            "jwsSignature": "eyJhbGciOiJFZERTQSJ9.nested-leak-attempt",
        }
    });

    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);

    assert!(
        decision.filtered_data["sectorData"]
            .get("jwsSignature")
            .is_none(),
        "nested jwsSignature must be redacted at Public tier"
    );
    // The surrounding object is still present (the redaction is field-level, not subtree-level)
    assert!(decision.filtered_data.get("sectorData").is_some());
}

// ─── Transfer state machine tests ─────────────────────────────────────────────

fn make_operator(did: &str, role: dpp_domain::OperatorRole) -> dpp_domain::ResponsibleOperator {
    dpp_domain::ResponsibleOperator {
        did: did.into(),
        name: "Test".into(),
        role,
        eu_operator_id: None,
        country: "DE".into(),
    }
}

fn make_initiated_transfer(
    passport_id: PassportId,
    from: &dpp_domain::ResponsibleOperator,
    to: &dpp_domain::ResponsibleOperator,
) -> TransferRecord {
    TransferRecord {
        transfer_id: Uuid::now_v7(),
        passport_id,
        from_operator: from.clone(),
        to_operator: to.clone(),
        reason: TransferReason::Sale,
        from_signature: Some("sig-from".into()),
        to_signature: None,
        initiated_at: Utc::now(),
        completed_at: None,
        rejected_at: None,
        cancelled_at: None,
        notes: None,
    }
}

/// After the incoming operator rejects a pending transfer, the outgoing
/// operator can initiate a new one — the chain is no longer blocked.
#[test]
fn transfer_reject_unblocks_chain() {
    use dpp_domain::OperatorRole;

    let pid = PassportId::new();
    let manufacturer = make_operator("did:web:maker.com", OperatorRole::Manufacturer);
    let buyer_a = make_operator("did:web:buyer-a.com", OperatorRole::Distributor);
    let buyer_b = make_operator("did:web:buyer-b.com", OperatorRole::Distributor);

    let mut chain = TransferChain::new(pid, manufacturer.clone());

    // Initiate transfer → Buyer A
    let t1 = make_initiated_transfer(pid, &manufacturer, &buyer_a);
    chain
        .initiate_transfer(t1)
        .expect("initiate first transfer");
    assert_eq!(
        chain.transfers.last().unwrap().status(),
        TransferStatus::Initiated
    );

    // Buyer A rejects
    chain
        .transfers
        .last_mut()
        .unwrap()
        .reject()
        .expect("reject");
    assert_eq!(
        chain.transfers.last().unwrap().status(),
        TransferStatus::Rejected
    );

    // Chain must accept a new transfer — rejected record is terminal, not pending
    let t2 = make_initiated_transfer(pid, &manufacturer, &buyer_b);
    chain
        .initiate_transfer(t2)
        .expect("chain must be unblocked after rejection");
    assert_eq!(chain.transfers.len(), 2);
}

/// After the outgoing operator cancels a pending transfer, a new transfer
/// can be initiated — the chain is not permanently blocked.
#[test]
fn transfer_cancel_unblocks_chain() {
    use dpp_domain::OperatorRole;

    let pid = PassportId::new();
    let manufacturer = make_operator("did:web:maker.com", OperatorRole::Manufacturer);
    let buyer = make_operator("did:web:buyer.com", OperatorRole::Distributor);

    let mut chain = TransferChain::new(pid, manufacturer.clone());

    // Initiate and then cancel
    let t1 = make_initiated_transfer(pid, &manufacturer, &buyer);
    chain.initiate_transfer(t1).expect("initiate");
    chain
        .transfers
        .last_mut()
        .unwrap()
        .cancel()
        .expect("cancel");
    assert_eq!(
        chain.transfers.last().unwrap().status(),
        TransferStatus::Cancelled
    );

    // Should allow a new transfer
    let t2 = make_initiated_transfer(pid, &manufacturer, &buyer);
    chain
        .initiate_transfer(t2)
        .expect("chain must be unblocked after cancellation");
}

/// Rejecting a completed transfer (wrong state) must return InvalidState.
#[test]
fn reject_completed_transfer_returns_invalid_state() {
    use dpp_domain::OperatorRole;

    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", OperatorRole::Distributor);

    let mut t = make_initiated_transfer(pid, &from, &to);
    t.to_signature = Some("sig-to".into());
    t.completed_at = Some(Utc::now());
    assert_eq!(t.status(), TransferStatus::Completed);

    assert!(
        matches!(t.reject(), Err(TransferError::InvalidState { .. })),
        "rejecting a completed transfer must return InvalidState"
    );
}

// ─── Passport sector-data validation tests ────────────────────────────────────

/// `Passport::validate()` must surface cross-field errors from `validate_sector_data`:
/// a Textile passport where fibre percentages do not sum to ~100% is invalid.
#[test]
fn passport_validate_catches_bad_fibre_sum() {
    let now = Utc::now();
    let mut passport = Passport {
        id: PassportId::new(),
        batch_id: None,
        product_name: "Test Shirt".into(),
        sector: Sector::Textile,
        product_category: None,
        manufacturer: ManufacturerInfo {
            name: "Factory GmbH".into(),
            address: "Berlin, DE".into(),
            did_web_url: None,
        },
        materials: vec![],
        co2e_per_unit: None,
        repairability_score: None,
        compliance_result: None,
        sector_data: Some(SectorData::Textile(TextileData {
            // sum = 50%, should be ~100%
            fibre_composition: vec![FibreEntry {
                fibre: "cotton".into(),
                pct: 50.0,
                country_of_origin: None,
            }],
            country_of_manufacturing: "DE".into(),
            care_instructions: "Machine wash 30°C".into(),
            chemical_compliance_standard: "REACH".into(),
            recycled_content_pct: None,
            carbon_footprint_kg_co2e: None,
            water_use_litres: None,
            microplastic_shedding_mg_per_wash: None,
            repair_score: None,
            durability_score: None,
            expected_wash_cycles: None,
            country_of_raw_material_origin: None,
            svhc_substances: None,
            allergens: None,
            substances_of_concern: None,
            recyclability_class: None,
            end_of_life_instructions: None,
            reuse_condition: None,
            prior_use_cycles: None,
            disassembly_instructions: None,
            spare_parts_available: None,
            product_weight_grams: None,
            repair_history_url: None,
            repair_count: None,
            pef_score: None,
        })),
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        created_at: now,
        updated_at: now,
        published_at: None,
        schema_version: "1.1.0".into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        retention_until: None,
        product_id: None,
        operator_identifier: None,
        facility_id: None,
    };

    let err = passport.validate().unwrap_err().to_string();
    assert!(
        err.contains("fibre") || err.contains("Fibre") || err.contains("50"),
        "validate() must surface fibre-sum error from sector_data validation, got: {err}"
    );

    // Fix the fibre sum and confirm validate() now passes.
    if let Some(SectorData::Textile(ref mut td)) = passport.sector_data {
        td.fibre_composition.push(FibreEntry {
            fibre: "polyester".into(),
            pct: 50.0,
            country_of_origin: None,
        });
    }
    assert!(
        passport.validate().is_ok(),
        "passport with valid fibre sum must pass validate()"
    );
}
