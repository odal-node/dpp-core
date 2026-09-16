//! Submission outcome and receipt.

use super::*;
use crate::RegistryStatusCode;

/// `SUCCESS`/`FAILURE`/`PROCESSING` is what became of a *submission*;
/// `RegistryStatusCode` is what became of a *record*. Distinct types, because a
/// failed submission produces no records for a record status to describe.
#[test]
fn submission_outcome_and_record_status_are_separate_vocabularies() {
    for (outcome, wire) in [
        (SubmissionOutcome::Processing, "PROCESSING"),
        (SubmissionOutcome::Success, "SUCCESS"),
        (SubmissionOutcome::Failure, "FAILURE"),
    ] {
        assert_eq!(outcome.wire_str(), wire);
        assert_eq!(serde_json::to_value(outcome).unwrap(), wire);
    }

    // The record vocabulary is unchanged and does not overlap.
    let record = serde_json::to_value(RegistryStatusCode::Registered).unwrap();
    assert_eq!(record, "registered");
    for outcome in [
        SubmissionOutcome::Processing,
        SubmissionOutcome::Success,
        SubmissionOutcome::Failure,
    ] {
        assert_ne!(serde_json::to_value(outcome).unwrap(), record);
    }
}

/// A poller asks one question. `Processing` is the only non-terminal state, and
/// getting that backwards either spins forever or abandons a live submission.
#[test]
fn only_processing_is_non_terminal() {
    assert!(!SubmissionOutcome::Processing.is_terminal());
    assert!(SubmissionOutcome::Success.is_terminal());
    assert!(SubmissionOutcome::Failure.is_terminal());
}

#[test]
fn submission_receipt_round_trips() {
    let receipt = SubmissionReceipt {
        correlation_id: "corr-8f31".into(),
        outcome: SubmissionOutcome::Processing,
    };
    let json = serde_json::to_value(&receipt).unwrap();
    assert_eq!(json["correlationId"], "corr-8f31");
    assert_eq!(json["outcome"], "PROCESSING");
    assert_eq!(
        serde_json::from_value::<SubmissionReceipt>(json).unwrap(),
        receipt
    );
}
