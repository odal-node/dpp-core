//! Example: Issue a Verifiable Credential and transfer passport responsibility.
//!
//! Demonstrates the W3C VC audience model and the dual-signature
//! transfer-of-responsibility chain.
//!
//! Run with: `cargo run --example credential_and_transfer`

use chrono::Utc;
use dpp_domain::{
    OperatorRole, PassportId, ResponsibleOperator, TransferChain, TransferReason, TransferRecord,
};
use dpp_vc::credential::{
    CredentialBuilder, CredentialRole, DppCredentialSubject, verify_credential_claims,
};
use uuid::Uuid;

fn main() {
    println!("=== W3C Verifiable Credential for DPP Access ===\n");

    // 1. Issue a credential to an authorised repairer
    let subject = DppCredentialSubject {
        id: "did:web:repair-shop.greenfix.de".into(),
        name: "GreenFix Textile Repair GmbH".into(),
        role: CredentialRole::AuthorisedRepairer,
        country: "DE".into(),
        product_groups: vec!["textile".into()],
        product_categories: vec![],
    };

    let credential =
        CredentialBuilder::new("did:web:authority.trade-registry.europa.eu".into(), subject)
            .expires_in_days(365)
            .build();

    println!("Issued credential: {}", credential.id);
    println!("  Issuer: {}", credential.issuer);
    println!("  Holder: {}", credential.credential_subject.id);
    println!("  Role: {:?}", credential.credential_subject.role);
    println!("  Expires: {}", credential.valid_until);

    // 2. Verify credential claims (structural + expiration — no signature check)
    let result = verify_credential_claims(&credential, Some("textile"), Utc::now());
    println!("\nVerification: {:?}", result);
    assert!(result.is_valid());

    // 3. Verify wrong product group is rejected
    let out_of_scope = verify_credential_claims(&credential, Some("battery"), Utc::now());
    println!("Out-of-scope check: {:?}", out_of_scope);
    assert!(!out_of_scope.is_valid());

    println!("\n=== Transfer of Responsibility ===\n");

    // 4. Set up a transfer chain
    let passport_id = PassportId::new();
    let manufacturer = ResponsibleOperator {
        did: "did:web:greenthread.example.com".into(),
        name: "GreenThread GmbH".into(),
        role: OperatorRole::Manufacturer,
        eu_operator_id: Some("DE811234567".into()),
        eu_operator_id_scheme: Some("vat".into()),
        country: "DE".into(),
    };

    let mut chain = TransferChain::new(passport_id, manufacturer.clone());
    println!(
        "Current operator: {} ({})",
        chain.current_operator().name,
        chain.current_operator().did
    );

    // 5. Transfer to a remanufacturer (dual-signature)
    let remanufacturer = ResponsibleOperator {
        did: "did:web:circular-tex.example.nl".into(),
        name: "CircularTex BV".into(),
        role: OperatorRole::Remanufacturer,
        eu_operator_id: Some("NL123456789B01".into()),
        eu_operator_id_scheme: Some("vat".into()),
        country: "NL".into(),
    };

    let transfer = TransferRecord {
        transfer_id: Uuid::now_v7(),
        passport_id,
        from_operator: manufacturer.clone(),
        to_operator: remanufacturer.clone(),
        reason: TransferReason::Remanufacturing,
        from_signature: Some("eyJhbGciOiJFZERTQSJ9.from-sig-placeholder".into()),
        node_acceptance_attestation: Some("eyJhbGciOiJFZERTQSJ9.to-sig-placeholder".into()),
        initiated_at: Utc::now(),
        completed_at: Some(Utc::now()),
        rejected_at: None,
        cancelled_at: None,
        notes: Some("Product remanufactured under circular economy programme".into()),
    };

    chain.initiate_transfer(transfer).unwrap();
    println!("\n✓ Transfer completed:");
    println!(
        "  New operator: {} ({})",
        chain.current_operator().name,
        chain.current_operator().did
    );
    println!("  Transfer count: {}", chain.transfer_count());
    println!("  Reason: Remanufacturing");

    // 6. Serialise chain for audit trail
    let json = serde_json::to_string_pretty(&chain).unwrap();
    println!(
        "\nTransfer chain JSON (first 400 chars):\n{}…",
        &json[..json.len().min(400)]
    );
}
