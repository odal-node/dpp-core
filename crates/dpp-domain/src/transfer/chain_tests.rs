//! [`TransferChain`] validity: the check on read and on append, the pending-record methods, and the JSON shape.

use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{TransferChain, TransferError, TransferReason, TransferRecord, TransferStatus};
use crate::operator::{OperatorRole, ResponsibleOperator};
use crate::passport::PassportId;

pub(super) const ATTESTATION: &str = "eyJhbGciOiJFZERTQSJ9.attestation";

pub(super) fn operator(did: &str) -> ResponsibleOperator {
    ResponsibleOperator {
        did: did.into(),
        name: did.into(),
        role: OperatorRole::Manufacturer,
        eu_operator_id: None,
        eu_operator_id_scheme: None,
        identifier: None,
        country: "DE".into(),
        registered_trade_name: None,
        postal_address: None,
        electronic_address: None,
    }
}

/// A transfer from `from` to `to`, authorised but not yet accepted.
pub(super) fn record(
    passport_id: PassportId,
    from: &ResponsibleOperator,
    to: &ResponsibleOperator,
) -> TransferRecord {
    TransferRecord {
        transfer_id: Uuid::now_v7(),
        passport_id,
        from_operator: from.clone(),
        to_operator: to.clone(),
        reason: TransferReason::Sale,
        from_signature: Some("eyJhbGciOiJFZERTQSJ9.from".into()),
        node_acceptance_attestation: None,
        initiated_at: Utc::now(),
        completed_at: None,
        rejected_at: None,
        cancelled_at: None,
        notes: None,
    }
}

/// Appends a transfer from the chain's current operator to `to`, still pending.
pub(super) fn append_pending(chain: &mut TransferChain, to: &ResponsibleOperator) {
    let passport_id = chain.passport_id();
    let from = chain.current_operator().clone();
    chain
        .initiate_transfer(record(passport_id, &from, to))
        .unwrap();
}

/// Appends a transfer from the chain's current operator to `to`, and completes it.
pub(super) fn append_completed(chain: &mut TransferChain, to: &ResponsibleOperator) {
    append_pending(chain, to);
    chain.complete_pending(ATTESTATION.into()).unwrap();
}

pub(super) fn to_json(chain: &TransferChain) -> Value {
    serde_json::to_value(chain).unwrap()
}

/// Reads a chain that must be refused, and returns the error text.
pub(super) fn refused(value: Value) -> String {
    serde_json::from_value::<TransferChain>(value)
        .expect_err("a chain that breaks the invariant must not read")
        .to_string()
}

#[test]
fn a_stored_chain_with_a_record_for_another_passport_is_refused() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a);
    append_completed(&mut chain, &b);
    let transfer_id = chain.records()[0].transfer_id;

    let mut value = to_json(&chain);
    value["transfers"][0]["passportId"] = json!(PassportId::new());

    let message = refused(value);
    assert!(
        message.contains(&pid.to_string()),
        "the message names the chain's passport: {message}"
    );
    assert!(
        message.contains("at record 0"),
        "the message names the record index: {message}"
    );
    assert!(
        message.contains(&transfer_id.to_string()),
        "the message names the record's transfer_id: {message}"
    );
    assert!(
        message.contains("passport mismatch"),
        "the message says what is wrong: {message}"
    );
}

#[test]
fn a_stored_chain_with_a_broken_operator_sequence_is_refused() {
    let pid = PassportId::new();
    let (a, b, c) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
    );
    let mut chain = TransferChain::new(pid, a.clone());
    append_completed(&mut chain, &b);
    append_completed(&mut chain, &c);
    let transfer_id = chain.records()[1].transfer_id;

    // The second hand-over now leaves from the original operator, not from B,
    // which is the operator the first hand-over put in place.
    let mut value = to_json(&chain);
    value["transfers"][1]["fromOperator"] = json!(a);

    let message = refused(value);
    assert!(
        message.contains("at record 1"),
        "the message names the record index: {message}"
    );
    assert!(
        message.contains(&transfer_id.to_string()),
        "the message names the record's transfer_id: {message}"
    );
    assert!(
        message.contains(&format!("expected {}, got {}", b.did, a.did)),
        "the message says which operator was expected: {message}"
    );
}

#[test]
fn a_stored_chain_whose_first_record_does_not_leave_the_original_operator_is_refused() {
    let pid = PassportId::new();
    let (a, b, c) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
    );
    let mut chain = TransferChain::new(pid, a.clone());
    append_completed(&mut chain, &b);

    let mut value = to_json(&chain);
    value["originalOperator"] = json!(c);

    let message = refused(value);
    assert!(
        message.contains("at record 0"),
        "the message names the record index: {message}"
    );
    assert!(
        message.contains(&format!("expected {}, got {}", c.did, a.did)),
        "the message says which operator was expected: {message}"
    );
}

#[test]
fn a_stored_chain_with_two_pending_records_is_refused() {
    let pid = PassportId::new();
    let (a, b, c) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
    );
    let mut chain = TransferChain::new(pid, a.clone());
    append_pending(&mut chain, &b);

    let second = record(pid, &a, &c);
    let mut value = to_json(&chain);
    value["transfers"]
        .as_array_mut()
        .unwrap()
        .push(json!(second));

    let message = refused(value);
    assert!(
        message.contains("at record 1"),
        "the message names the record index: {message}"
    );
    assert!(
        message.contains(&second.transfer_id.to_string()),
        "the message names the record's transfer_id: {message}"
    );
    assert!(
        message.contains("already pending"),
        "the message says what is wrong: {message}"
    );
}

#[test]
fn a_stored_chain_with_a_completed_record_after_a_pending_one_is_refused() {
    let pid = PassportId::new();
    let (a, b, c) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
    );
    let mut chain = TransferChain::new(pid, a.clone());
    append_pending(&mut chain, &b);

    let mut after = record(pid, &a, &c);
    after.node_acceptance_attestation = Some(ATTESTATION.into());
    after.completed_at = Some(Utc::now());
    let mut value = to_json(&chain);
    value["transfers"]
        .as_array_mut()
        .unwrap()
        .push(json!(after));

    let message = refused(value);
    assert!(
        message.contains("at record 1"),
        "the message names the record index: {message}"
    );
    assert!(
        message.contains(&after.transfer_id.to_string()),
        "the message names the record's transfer_id: {message}"
    );
}

#[test]
fn a_chain_built_through_the_methods_round_trips_unchanged() {
    let pid = PassportId::new();
    let (a, b, c, d, e) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
        operator("did:web:d.example"),
        operator("did:web:e.example"),
    );
    let mut chain = TransferChain::new(pid, a.clone());
    append_completed(&mut chain, &b);
    append_pending(&mut chain, &c);
    chain.reject_pending().unwrap();
    append_pending(&mut chain, &d);
    chain.cancel_pending().unwrap();
    append_completed(&mut chain, &e);
    append_pending(&mut chain, &c);

    let json = to_json(&chain);
    let back: TransferChain = serde_json::from_value(json.clone()).unwrap();

    assert_eq!(to_json(&back), json);
    assert_eq!(back.current_operator().did, e.did);
    assert_eq!(back.transfer_count(), 2);
    assert_eq!(back.pending().unwrap().to_operator.did, c.did);

    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["originalOperator", "passportId", "transfers"]);
}

#[test]
fn reject_pending_refuses_an_accepted_record_and_leaves_it_pending() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a.clone());
    let mut accepted = record(pid, &a, &b);
    accepted.node_acceptance_attestation = Some(ATTESTATION.into());
    chain.initiate_transfer(accepted).unwrap();
    let before = to_json(&chain);

    assert_eq!(
        chain.reject_pending().unwrap_err(),
        TransferError::InvalidState {
            current: TransferStatus::Accepted,
            action: "reject".into(),
        }
    );
    assert_eq!(to_json(&chain), before);
    assert_eq!(chain.pending().unwrap().status(), TransferStatus::Accepted);
}

#[test]
fn each_pending_method_refuses_when_nothing_is_pending_and_leaves_the_chain_alone() {
    let pid = PassportId::new();
    let (a, b, c) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
    );

    let mut empty = TransferChain::new(pid, a.clone());
    assert_eq!(
        empty.reject_pending().unwrap_err(),
        TransferError::NoPendingTransfer
    );
    assert_eq!(
        empty.cancel_pending().unwrap_err(),
        TransferError::NoPendingTransfer
    );
    assert_eq!(
        empty.complete_pending(ATTESTATION.into()).unwrap_err(),
        TransferError::NoPendingTransfer
    );

    // A chain whose last record is terminal has nothing pending either.
    let mut chain = TransferChain::new(pid, a.clone());
    append_completed(&mut chain, &b);
    append_pending(&mut chain, &c);
    chain.reject_pending().unwrap();
    let before = to_json(&chain);

    assert!(chain.pending().is_none());
    assert_eq!(
        chain.reject_pending().unwrap_err(),
        TransferError::NoPendingTransfer
    );
    assert_eq!(
        chain.cancel_pending().unwrap_err(),
        TransferError::NoPendingTransfer
    );
    assert_eq!(
        chain.complete_pending(ATTESTATION.into()).unwrap_err(),
        TransferError::NoPendingTransfer
    );
    assert_eq!(to_json(&chain), before);
}

#[test]
fn complete_pending_refuses_a_record_without_a_from_signature_and_leaves_it_pending() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a.clone());
    let mut unsigned = record(pid, &a, &b);
    unsigned.from_signature = None;
    chain.initiate_transfer(unsigned).unwrap();
    let before = to_json(&chain);

    assert_eq!(
        chain.complete_pending(ATTESTATION.into()).unwrap_err(),
        TransferError::InvalidState {
            current: TransferStatus::Initiated,
            action: "complete".into(),
        }
    );
    assert_eq!(to_json(&chain), before);
    assert!(
        chain
            .pending()
            .unwrap()
            .node_acceptance_attestation
            .is_none()
    );
}

#[test]
fn complete_pending_never_replaces_an_existing_attestation() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a.clone());
    let mut accepted = record(pid, &a, &b);
    accepted.node_acceptance_attestation = Some("first".into());
    chain.initiate_transfer(accepted).unwrap();
    let before = to_json(&chain);

    assert_eq!(
        chain.complete_pending("second".into()).unwrap_err(),
        TransferError::InvalidState {
            current: TransferStatus::Accepted,
            action: "replace the acceptance attestation".into(),
        }
    );
    assert_eq!(to_json(&chain), before);
}

#[test]
fn complete_pending_with_the_same_attestation_completes_the_record() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a.clone());
    let mut accepted = record(pid, &a, &b);
    accepted.node_acceptance_attestation = Some("first".into());
    chain.initiate_transfer(accepted).unwrap();

    let done = chain.complete_pending("first".into()).unwrap();
    assert_eq!(done.status(), TransferStatus::Completed);
    assert_eq!(chain.current_operator().did, b.did);
}

#[test]
fn initiate_transfer_still_returns_the_bare_errors_and_leaves_the_chain_alone() {
    let pid = PassportId::new();
    let other = PassportId::new();
    let (a, b, c) = (
        operator("did:web:a.example"),
        operator("did:web:b.example"),
        operator("did:web:c.example"),
    );
    let mut chain = TransferChain::new(pid, a.clone());

    assert_eq!(
        chain.initiate_transfer(record(other, &a, &b)).unwrap_err(),
        TransferError::PassportMismatch {
            expected: pid.to_string(),
            got: other.to_string(),
        }
    );
    assert_eq!(
        chain.initiate_transfer(record(pid, &b, &c)).unwrap_err(),
        TransferError::OperatorMismatch {
            expected: a.did.clone(),
            got: b.did.clone(),
        }
    );
    assert!(chain.records().is_empty());

    chain.initiate_transfer(record(pid, &a, &b)).unwrap();
    assert_eq!(
        chain.initiate_transfer(record(pid, &a, &c)).unwrap_err(),
        TransferError::TransferAlreadyPending
    );
    assert_eq!(chain.records().len(), 1);
}

#[test]
fn a_valid_stored_chain_under_the_old_acceptance_key_still_reads() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a);
    append_completed(&mut chain, &b);

    let mut value = to_json(&chain);
    let transfer = value["transfers"][0].as_object_mut().unwrap();
    let attestation = transfer.remove("nodeAcceptanceAttestation").unwrap();
    transfer.insert("toSignature".into(), attestation);

    let back: TransferChain = serde_json::from_value(value).expect("a valid chain must read");
    assert!(back.records()[0].is_complete());
    assert_eq!(back.current_operator().did, b.did);
}
