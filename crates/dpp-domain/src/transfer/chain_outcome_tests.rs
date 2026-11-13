//! The outcome rules of a transfer chain: one outcome per record, and both signatures on a completion.

use chrono::Utc;
use serde_json::json;

use super::chain_tests::{
    ATTESTATION, append_completed, append_pending, operator, record, refused, to_json,
};
use super::{TransferChain, TransferError, TransferStatus};
use crate::passport::PassportId;

fn reads_back(chain: &TransferChain) {
    serde_json::from_value::<TransferChain>(to_json(chain))
        .expect("a chain built through the methods must read back");
}

#[test]
fn a_stored_record_with_two_outcomes_is_refused_on_read() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a.clone());
    append_completed(&mut chain, &b);
    append_pending(&mut chain, &a);
    chain.reject_pending().unwrap();
    for (index, key) in [(0, "rejectedAt"), (0, "cancelledAt"), (1, "cancelledAt")] {
        let mut value = to_json(&chain);
        value["transfers"][index][key] = json!(Utc::now());
        let message = refused(value);
        assert!(
            message.contains(&format!(
                "at record {index} ({})",
                chain.records()[index].transfer_id
            )) && message.contains("more than one outcome"),
            "{message}"
        );
    }
}

#[test]
fn initiate_transfer_refuses_a_record_with_two_outcomes_and_leaves_the_chain_alone() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a.clone());
    append_completed(&mut chain, &b);
    let mut both = record(pid, &b, &a);
    both.node_acceptance_attestation = Some(ATTESTATION.into());
    both.completed_at = Some(Utc::now());
    both.rejected_at = Some(Utc::now());
    let before = to_json(&chain);
    assert_eq!(
        chain.initiate_transfer(both).unwrap_err(),
        TransferError::InvalidState {
            current: TransferStatus::Rejected,
            action: "record more than one outcome".into(),
        }
    );
    assert_eq!(to_json(&chain), before);
    assert_eq!(chain.current_operator().did, b.did);
}

#[test]
fn a_stored_completion_without_both_signatures_is_refused_on_read() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a);
    append_pending(&mut chain, &b);
    let mut value = to_json(&chain);
    value["transfers"][0]["completedAt"] = json!(Utc::now());
    let message = refused(value);
    assert!(
        message.contains("without both signatures")
            && message.contains(&chain.records()[0].transfer_id.to_string()),
        "{message}"
    );
}

#[test]
fn initiate_transfer_refuses_a_completion_without_both_signatures_and_leaves_the_chain_alone() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut chain = TransferChain::new(pid, a.clone());
    let mut partial = record(pid, &a, &b);
    partial.completed_at = Some(Utc::now());
    let before = to_json(&chain);
    assert_eq!(
        chain.initiate_transfer(partial).unwrap_err(),
        TransferError::InvalidState {
            current: TransferStatus::Initiated,
            action: "record a completion without both signatures".into(),
        }
    );
    assert_eq!(to_json(&chain), before);
}

/// Such a completion never reaches a chain, so neither pending method can act on one,
/// and every chain built through the methods reads back after each step.
#[test]
fn no_pending_method_leaves_a_chain_that_cannot_be_read_back() {
    let pid = PassportId::new();
    let (a, b) = (operator("did:web:a.example"), operator("did:web:b.example"));
    let mut partial = record(pid, &a, &b);
    partial.completed_at = Some(Utc::now());
    let mut chain = TransferChain::new(pid, a.clone());
    assert!(chain.initiate_transfer(partial).is_err());
    assert_eq!(
        chain.reject_pending().unwrap_err(),
        TransferError::NoPendingTransfer
    );
    assert_eq!(
        chain.cancel_pending().unwrap_err(),
        TransferError::NoPendingTransfer
    );

    append_pending(&mut chain, &b);
    chain.reject_pending().unwrap();
    reads_back(&chain);
    append_pending(&mut chain, &b);
    chain.cancel_pending().unwrap();
    reads_back(&chain);
    append_pending(&mut chain, &b);
    chain.complete_pending(ATTESTATION.into()).unwrap();
    reads_back(&chain);
}
