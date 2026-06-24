//! Integration test: Transfer of Responsibility lifecycle.
//!
//! Exercises the complete transfer flow across dpp-domain types:
//!
//! 1. Create a passport and its initial TransferChain (manufacturer)
//! 2. Initiate a transfer to a remanufacturer
//! 3. Complete the transfer (both parties sign)
//! 4. Verify the chain's current operator changes correctly
//! 5. Chain a second transfer (remanufacturer → distributor)
//! 6. Validate rejection of invalid transfers (wrong operator, duplicate pending)
//! 7. Verify serialisation round-trip of the full chain

use chrono::Utc;
use dpp_domain::{
    OperatorRole, PassportId, ResponsibleOperator, TransferChain, TransferError, TransferReason,
    TransferRecord, TransferStatus,
};
use uuid::Uuid;

fn make_operator(did: &str, name: &str, role: OperatorRole, country: &str) -> ResponsibleOperator {
    ResponsibleOperator {
        did: did.into(),
        name: name.into(),
        role,
        eu_operator_id: None,
        country: country.into(),
    }
}

fn make_transfer(
    passport_id: PassportId,
    from: &ResponsibleOperator,
    to: &ResponsibleOperator,
    reason: TransferReason,
) -> TransferRecord {
    TransferRecord {
        transfer_id: Uuid::now_v7(),
        passport_id,
        from_operator: from.clone(),
        to_operator: to.clone(),
        reason,
        from_signature: Some("eyJhbGciOiJFZERTQSJ9.from-sig".into()),
        to_signature: None,
        initiated_at: Utc::now(),
        completed_at: None,
        rejected_at: None,
        cancelled_at: None,
        notes: None,
    }
}

fn complete_transfer(record: &mut TransferRecord) {
    record.to_signature = Some("eyJhbGciOiJFZERTQSJ9.to-sig".into());
    record.completed_at = Some(Utc::now());
}

// ─── Full lifecycle test ──────────────────────────────────────────────────

#[test]
fn full_transfer_lifecycle_manufacturer_to_remanufacturer_to_distributor() {
    let pid = PassportId::new();

    // Step 1: Manufacturer creates the passport
    let manufacturer = make_operator(
        "did:web:ecotextile.example.com",
        "EcoTextile GmbH",
        OperatorRole::Manufacturer,
        "DE",
    );
    let mut chain = TransferChain::new(pid, manufacturer.clone());
    assert_eq!(
        chain.current_operator().did,
        "did:web:ecotextile.example.com"
    );
    assert_eq!(chain.transfer_count(), 0);

    // Step 2: Transfer to remanufacturer
    let remanufacturer = make_operator(
        "did:web:remaker.example.com",
        "ReMaker Industries",
        OperatorRole::Remanufacturer,
        "NL",
    );
    let transfer1 = make_transfer(
        pid,
        &manufacturer,
        &remanufacturer,
        TransferReason::Remanufacturing,
    );

    // Before completion, current operator is still the manufacturer
    chain.initiate_transfer(transfer1.clone()).unwrap();
    assert_eq!(
        chain.current_operator().did,
        "did:web:ecotextile.example.com",
        "incomplete transfer should not change current operator"
    );

    // Step 3: Complete transfer (incoming operator signs)
    // We need to modify the last transfer in the chain directly
    // since initiate_transfer moved it in
    let last = chain.transfers.last_mut().unwrap();
    complete_transfer(last);
    assert_eq!(last.status(), TransferStatus::Completed);

    // Step 4: Current operator should now be the remanufacturer
    assert_eq!(
        chain.current_operator().did,
        "did:web:remaker.example.com",
        "completed transfer should update current operator"
    );
    assert_eq!(chain.transfer_count(), 1);

    // Step 5: Chain a second transfer to a distributor
    let distributor = make_operator(
        "did:web:greendist.example.com",
        "GreenDist BV",
        OperatorRole::Distributor,
        "BE",
    );
    let mut transfer2 = make_transfer(pid, &remanufacturer, &distributor, TransferReason::Sale);
    complete_transfer(&mut transfer2);
    chain.initiate_transfer(transfer2).unwrap();

    assert_eq!(
        chain.current_operator().did,
        "did:web:greendist.example.com"
    );
    assert_eq!(chain.transfer_count(), 2);
}

// ─── Error cases ──────────────────────────────────────────────────────────

#[test]
fn transfer_from_wrong_operator_rejected() {
    let pid = PassportId::new();
    let manufacturer = make_operator(
        "did:web:acme.example.com",
        "ACME",
        OperatorRole::Manufacturer,
        "DE",
    );
    let mut chain = TransferChain::new(pid, manufacturer);

    // An impostor tries to initiate a transfer
    let impostor = make_operator(
        "did:web:impostor.example.com",
        "Impostor Co",
        OperatorRole::Importer,
        "CN",
    );
    let target = make_operator(
        "did:web:target.example.com",
        "Target Ltd",
        OperatorRole::Distributor,
        "FR",
    );
    let transfer = make_transfer(pid, &impostor, &target, TransferReason::Sale);

    let err = chain.initiate_transfer(transfer).unwrap_err();
    assert!(
        matches!(err, TransferError::OperatorMismatch { .. }),
        "transfer from wrong operator must be rejected"
    );
}

#[test]
fn pending_transfer_blocks_new_initiation() {
    let pid = PassportId::new();
    let manufacturer = make_operator(
        "did:web:factory.example.com",
        "Factory Inc",
        OperatorRole::Manufacturer,
        "IT",
    );
    let mut chain = TransferChain::new(pid, manufacturer.clone());

    let target_a = make_operator(
        "did:web:buyer-a.example.com",
        "Buyer A",
        OperatorRole::Distributor,
        "ES",
    );
    let target_b = make_operator(
        "did:web:buyer-b.example.com",
        "Buyer B",
        OperatorRole::Distributor,
        "PT",
    );

    // First transfer — pending (no to_signature)
    let transfer_a = make_transfer(pid, &manufacturer, &target_a, TransferReason::Sale);
    chain.initiate_transfer(transfer_a).unwrap();

    // Second transfer should be blocked
    let transfer_b = make_transfer(pid, &manufacturer, &target_b, TransferReason::Sale);
    let err = chain.initiate_transfer(transfer_b).unwrap_err();
    assert!(
        matches!(err, TransferError::TransferAlreadyPending),
        "cannot initiate a second transfer while one is pending"
    );
}

// ─── Serialisation ────────────────────────────────────────────────────────

#[test]
fn transfer_chain_serialisation_round_trip() {
    let pid = PassportId::new();
    let manufacturer = make_operator(
        "did:web:acme.example.com",
        "ACME Textiles",
        OperatorRole::Manufacturer,
        "DE",
    );
    let distributor = make_operator(
        "did:web:dist.example.com",
        "Euro Dist",
        OperatorRole::Distributor,
        "FR",
    );
    let mut chain = TransferChain::new(pid, manufacturer.clone());

    let mut transfer = make_transfer(pid, &manufacturer, &distributor, TransferReason::Sale);
    complete_transfer(&mut transfer);
    chain.initiate_transfer(transfer).unwrap();

    let json = serde_json::to_value(&chain).unwrap();
    let back: TransferChain = serde_json::from_value(json).unwrap();

    assert_eq!(back.passport_id, pid);
    assert_eq!(back.original_operator.did, "did:web:acme.example.com");
    assert_eq!(back.transfers.len(), 1);
    assert_eq!(back.current_operator().did, "did:web:dist.example.com");
}

#[test]
fn transfer_provenance_audit_trail() {
    let pid = PassportId::new();
    let op1 = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer, "DE");
    let op2 = make_operator("did:web:b.com", "B", OperatorRole::Remanufacturer, "NL");
    let op3 = make_operator("did:web:c.com", "C", OperatorRole::Distributor, "FR");
    let op4 = make_operator("did:web:d.com", "D", OperatorRole::PreparerForReuse, "BE");

    let mut chain = TransferChain::new(pid, op1.clone());

    // Build a 3-hop chain
    for (from, to, reason) in [
        (&op1, &op2, TransferReason::Remanufacturing),
        (&op2, &op3, TransferReason::Sale),
        (&op3, &op4, TransferReason::PreparationForReuse),
    ] {
        let mut t = make_transfer(pid, from, to, reason);
        complete_transfer(&mut t);
        chain.initiate_transfer(t).unwrap();
    }

    assert_eq!(chain.transfer_count(), 3);
    assert_eq!(chain.current_operator().did, "did:web:d.com");

    // Every completed transfer should have both signatures and a completion timestamp
    for t in &chain.transfers {
        assert!(t.is_complete());
        assert!(t.from_signature.is_some());
        assert!(t.to_signature.is_some());
        assert!(t.completed_at.is_some());
    }
}
