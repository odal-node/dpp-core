//! Transfer state-machine, chain-tracking, and serde tests.

use super::*;

fn make_operator(did: &str, name: &str, role: OperatorRole) -> ResponsibleOperator {
    ResponsibleOperator {
        did: did.into(),
        name: name.into(),
        role,
        eu_operator_id: None,
        country: "DE".into(),
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
        from_signature: Some("sig-from".into()),
        to_signature: None,
        initiated_at: Utc::now(),
        completed_at: None,
        rejected_at: None,
        cancelled_at: None,
        notes: None,
    }
}

#[test]
fn new_chain_returns_original_operator() {
    let pid = PassportId::new();
    let op = make_operator("did:web:acme.com", "ACME", OperatorRole::Manufacturer);
    let chain = TransferChain::new(pid, op.clone());
    assert_eq!(chain.current_operator(), &op);
    assert_eq!(chain.transfer_count(), 0);
}

#[test]
fn completed_transfer_changes_current_operator() {
    let pid = PassportId::new();
    let original = make_operator("did:web:acme.com", "ACME", OperatorRole::Manufacturer);
    let new_op = make_operator(
        "did:web:remaker.com",
        "ReMaker",
        OperatorRole::Remanufacturer,
    );
    let mut chain = TransferChain::new(pid, original.clone());

    let mut transfer = make_transfer(pid, &original, &new_op, TransferReason::Remanufacturing);
    transfer.to_signature = Some("sig-to".into());
    transfer.completed_at = Some(Utc::now());

    chain.initiate_transfer(transfer).unwrap();
    assert_eq!(chain.current_operator().did, "did:web:remaker.com");
    assert_eq!(chain.transfer_count(), 1);
}

#[test]
fn operator_mismatch_rejected() {
    let pid = PassportId::new();
    let original = make_operator("did:web:acme.com", "ACME", OperatorRole::Manufacturer);
    let wrong = make_operator("did:web:wrong.com", "Wrong", OperatorRole::Importer);
    let target = make_operator("did:web:target.com", "Target", OperatorRole::Distributor);
    let mut chain = TransferChain::new(pid, original);

    let transfer = make_transfer(pid, &wrong, &target, TransferReason::Sale);
    let result = chain.initiate_transfer(transfer);
    assert!(matches!(
        result,
        Err(TransferError::OperatorMismatch { .. })
    ));
}

#[test]
fn pending_transfer_blocks_new_initiation() {
    let pid = PassportId::new();
    let original = make_operator("did:web:acme.com", "ACME", OperatorRole::Manufacturer);
    let target1 = make_operator("did:web:target1.com", "Target1", OperatorRole::Importer);
    let target2 = make_operator("did:web:target2.com", "Target2", OperatorRole::Distributor);
    let mut chain = TransferChain::new(pid, original.clone());

    // First transfer — initiated but not completed
    let transfer1 = make_transfer(pid, &original, &target1, TransferReason::Sale);
    chain.initiate_transfer(transfer1).unwrap();

    // Second transfer — should be rejected
    let transfer2 = make_transfer(pid, &original, &target2, TransferReason::Sale);
    let result = chain.initiate_transfer(transfer2);
    assert!(matches!(result, Err(TransferError::TransferAlreadyPending)));
}

#[test]
fn transfer_status_derives_correctly() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);

    // Initiated only
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    assert_eq!(t.status(), TransferStatus::Initiated);

    // Accepted (both signed, not completed)
    t.to_signature = Some("sig".into());
    assert_eq!(t.status(), TransferStatus::Accepted);

    // Completed
    t.completed_at = Some(Utc::now());
    assert_eq!(t.status(), TransferStatus::Completed);
    assert!(t.is_complete());
}

#[test]
fn multiple_completed_transfers_track_chain() {
    let pid = PassportId::new();
    let op_a = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let op_b = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let op_c = make_operator("did:web:c.com", "C", OperatorRole::Remanufacturer);
    let mut chain = TransferChain::new(pid, op_a.clone());

    // A → B
    let mut t1 = make_transfer(pid, &op_a, &op_b, TransferReason::Sale);
    t1.to_signature = Some("sig".into());
    t1.completed_at = Some(Utc::now());
    chain.initiate_transfer(t1).unwrap();
    assert_eq!(chain.current_operator().did, "did:web:b.com");

    // B → C
    let mut t2 = make_transfer(pid, &op_b, &op_c, TransferReason::Remanufacturing);
    t2.to_signature = Some("sig".into());
    t2.completed_at = Some(Utc::now());
    chain.initiate_transfer(t2).unwrap();
    assert_eq!(chain.current_operator().did, "did:web:c.com");
    assert_eq!(chain.transfer_count(), 2);
}

#[test]
fn serde_round_trip() {
    let pid = PassportId::new();
    let op = make_operator("did:web:acme.com", "ACME", OperatorRole::Manufacturer);
    let chain = TransferChain::new(pid, op);
    let json = serde_json::to_string(&chain).unwrap();
    let back: TransferChain = serde_json::from_str(&json).unwrap();
    assert_eq!(back.passport_id, pid);
    assert_eq!(back.original_operator.did, "did:web:acme.com");
}

#[test]
fn reject_from_initiated_succeeds() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    assert_eq!(t.status(), TransferStatus::Initiated);
    t.reject().unwrap();
    assert_eq!(t.status(), TransferStatus::Rejected);
    assert!(t.rejected_at.is_some());
}

#[test]
fn reject_from_accepted_fails() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    t.to_signature = Some("sig-to".into()); // now Accepted
    assert_eq!(t.status(), TransferStatus::Accepted);
    assert!(matches!(
        t.reject(),
        Err(TransferError::InvalidState { .. })
    ));
}

#[test]
fn cancel_from_initiated_succeeds() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    t.cancel().unwrap();
    assert_eq!(t.status(), TransferStatus::Cancelled);
}

#[test]
fn cancel_from_accepted_succeeds() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    t.to_signature = Some("sig-to".into());
    assert_eq!(t.status(), TransferStatus::Accepted);
    t.cancel().unwrap();
    assert_eq!(t.status(), TransferStatus::Cancelled);
}

#[test]
fn cancel_from_completed_fails() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    t.to_signature = Some("sig-to".into());
    t.completed_at = Some(Utc::now());
    assert_eq!(t.status(), TransferStatus::Completed);
    assert!(matches!(
        t.cancel(),
        Err(TransferError::InvalidState { .. })
    ));
}

#[test]
fn complete_from_accepted_succeeds() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    t.to_signature = Some("sig-to".into());
    t.complete().unwrap();
    assert_eq!(t.status(), TransferStatus::Completed);
    assert!(t.is_complete());
}

#[test]
fn complete_from_initiated_fails() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    // Only the from-signature is present → still Initiated, not Accepted.
    let mut t = make_transfer(pid, &from, &to, TransferReason::Sale);
    assert_eq!(t.status(), TransferStatus::Initiated);
    assert!(matches!(
        t.complete(),
        Err(TransferError::InvalidState { .. })
    ));
}

#[test]
fn transfer_error_display_messages() {
    let mismatch = TransferError::OperatorMismatch {
        expected: "did:web:a.com".into(),
        got: "did:web:b.com".into(),
    };
    assert_eq!(
        mismatch.to_string(),
        "operator mismatch: expected did:web:a.com, got did:web:b.com"
    );

    let pending = TransferError::TransferAlreadyPending;
    assert!(pending.to_string().contains("already pending"));

    let invalid = TransferError::InvalidState {
        current: TransferStatus::Completed,
        action: "cancel".into(),
    };
    assert!(invalid.to_string().contains("cannot cancel"));

    // Usable as a std::error::Error trait object.
    let boxed: Box<dyn std::error::Error> = Box::new(pending);
    assert!(!boxed.to_string().is_empty());
}

#[test]
fn rejected_transfer_unblocks_new_initiation() {
    let pid = PassportId::new();
    let op_a = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let op_b = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut chain = TransferChain::new(pid, op_a.clone());

    // Initiate and then reject
    let t1 = make_transfer(pid, &op_a, &op_b, TransferReason::Sale);
    chain.initiate_transfer(t1.clone()).unwrap();
    let t1_mut = chain.transfers.last_mut().unwrap();
    t1_mut.reject().unwrap();

    // Chain should allow a new transfer
    let t2 = make_transfer(pid, &op_a, &op_b, TransferReason::Sale);
    assert!(chain.initiate_transfer(t2).is_ok());
}

#[test]
fn cancelled_transfer_unblocks_new_initiation() {
    let pid = PassportId::new();
    let op_a = make_operator("did:web:a.com", "A", OperatorRole::Manufacturer);
    let op_b = make_operator("did:web:b.com", "B", OperatorRole::Importer);
    let mut chain = TransferChain::new(pid, op_a.clone());

    let t1 = make_transfer(pid, &op_a, &op_b, TransferReason::Sale);
    chain.initiate_transfer(t1).unwrap();
    chain.transfers.last_mut().unwrap().cancel().unwrap();

    let t2 = make_transfer(pid, &op_a, &op_b, TransferReason::Sale);
    assert!(chain.initiate_transfer(t2).is_ok());
}
