//! Transfer state-machine, chain-tracking, and serde tests.

use super::*;
use crate::passport::PassportId;
use chrono::Utc;
use uuid::Uuid;

fn make_operator(did: &str, name: &str, role: OperatorRole) -> ResponsibleOperator {
    ResponsibleOperator {
        did: did.into(),
        name: name.into(),
        role,
        eu_operator_id: None,
        eu_operator_id_scheme: None,
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
        node_acceptance_attestation: None,
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
    transfer.node_acceptance_attestation = Some("sig-to".into());
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
    t.node_acceptance_attestation = Some("sig".into());
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
    t1.node_acceptance_attestation = Some("sig".into());
    t1.completed_at = Some(Utc::now());
    chain.initiate_transfer(t1).unwrap();
    assert_eq!(chain.current_operator().did, "did:web:b.com");

    // B → C
    let mut t2 = make_transfer(pid, &op_b, &op_c, TransferReason::Remanufacturing);
    t2.node_acceptance_attestation = Some("sig".into());
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
    t.node_acceptance_attestation = Some("sig-to".into()); // now Accepted
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
    t.node_acceptance_attestation = Some("sig-to".into());
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
    t.node_acceptance_attestation = Some("sig-to".into());
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
    t.node_acceptance_attestation = Some("sig-to".into());
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

/// A chain stored before the rename must keep reading, with the acceptance
/// marker intact.
///
/// `TransferChain` is persisted as a JSON document, so renaming the field
/// renamed the wire key. Without the `toSignature` alias every already-completed
/// transfer would deserialize with `node_acceptance_attestation: None`, and
/// [`TransferRecord::is_complete`] would begin answering `false` for handovers
/// that did complete — a silent rewrite of history rather than a load error.
#[test]
fn a_record_stored_under_the_old_key_still_reads_as_complete() {
    let stored = serde_json::json!({
        "transferId": Uuid::now_v7(),
        "passportId": PassportId::new(),
        "fromOperator": make_operator("did:web:a.example", "A", OperatorRole::Manufacturer),
        "toOperator": make_operator("did:web:b.example", "B", OperatorRole::Remanufacturer),
        "reason": "sale",
        "fromSignature": "eyJhbGciOiJFZERTQSJ9.from",
        "toSignature": "eyJhbGciOiJFZERTQSJ9.from",
        "initiatedAt": "2026-07-01T00:00:00Z",
        "completedAt": "2026-07-02T00:00:00Z",
        "notes": null,
    });

    let record: TransferRecord = serde_json::from_value(stored).expect("old shape must read");

    assert_eq!(
        record.node_acceptance_attestation.as_deref(),
        Some("eyJhbGciOiJFZERTQSJ9.from"),
        "the alias did not carry the stored acceptance marker across the rename"
    );
    assert!(
        record.is_complete(),
        "a completed handover must not become incomplete by being re-read"
    );
    assert_eq!(record.status(), TransferStatus::Completed);
}

/// The new name is what we write, even though the old one is still read.
///
/// An alias is a one-way concession to stored data. If serialisation kept
/// emitting `toSignature` the rename would be cosmetic — every consumer
/// downstream, including the registry notification, would still be told that
/// the incoming operator signed.
#[test]
fn serialisation_emits_the_new_key_only() {
    let pid = PassportId::new();
    let from = make_operator("did:web:a.example", "A", OperatorRole::Manufacturer);
    let to = make_operator("did:web:b.example", "B", OperatorRole::Recycler);
    let mut record = make_transfer(pid, &from, &to, TransferReason::InsolvencySuccession);
    record.node_acceptance_attestation = Some("attestation".into());

    let wire = serde_json::to_value(&record).expect("serialise");

    assert!(
        wire.get("nodeAcceptanceAttestation").is_some(),
        "the honest key must be on the wire"
    );
    assert!(
        wire.get("toSignature").is_none(),
        "the old key must not be emitted — it states a claim the node cannot make"
    );
}
