//! Submission outcome, receipt and per-passport results.

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
        assert_eq!(serde_json::to_value(&outcome).unwrap(), wire);
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
        results: Vec::new(),
    };
    let json = serde_json::to_value(&receipt).unwrap();
    assert_eq!(json["correlationId"], "corr-8f31");
    assert_eq!(json["outcome"], "PROCESSING");
    assert_eq!(
        serde_json::from_value::<SubmissionReceipt>(json).unwrap(),
        receipt
    );
}

/// 🚨 A value the registry sends and this crate does not know is kept, not
/// refused, and nobody stops polling on it.
///
/// The three names are what the web interface shows. If the API says
/// `"QUEUED"` instead, a parse error would make a reply that may report a
/// successful registration indistinguishable from a broken one.
#[test]
fn an_unknown_outcome_is_carried_and_not_terminal() {
    let parsed: SubmissionOutcome = serde_json::from_value("QUEUED".into()).unwrap();
    assert_eq!(parsed, SubmissionOutcome::Unrecognised("QUEUED".into()));
    assert!(!parsed.is_terminal());
    assert_eq!(parsed.wire_str(), "QUEUED");
    assert_eq!(serde_json::to_value(&parsed).unwrap(), "QUEUED");

    // The known three still read as themselves, not as unknown.
    let known: SubmissionOutcome = serde_json::from_value("FAILURE".into()).unwrap();
    assert_eq!(known, SubmissionOutcome::Failure);
}

/// The same for the record vocabulary, which no published list backs at all.
#[test]
fn an_unknown_record_status_is_carried_rather_than_refused() {
    let parsed: RegistryStatusCode = serde_json::from_value("verification_lapsed".into()).unwrap();
    assert_eq!(
        parsed,
        RegistryStatusCode::Unrecognised("verification_lapsed".into())
    );
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        "verification_lapsed"
    );

    for (status, wire) in [
        (RegistryStatusCode::Pending, "pending"),
        (RegistryStatusCode::Registered, "registered"),
        (RegistryStatusCode::Rejected, "rejected"),
        (
            RegistryStatusCode::SuspendedByAuthority,
            "suspended_by_authority",
        ),
        (RegistryStatusCode::Deactivated, "deactivated"),
    ] {
        assert_eq!(status.wire_str(), wire);
        assert_eq!(serde_json::to_value(&status).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<RegistryStatusCode>(wire.into()).unwrap(),
            status
        );
    }
}

fn result(upi: &str) -> PassportResult {
    PassportResult {
        unique_product_identifier: upi.into(),
        product_group: Some("BATTERIES".into()),
        outcome: None,
        registration_identifier: None,
        error_code: None,
    }
}

/// 🚨 Results are found by identifier, never by position.
///
/// The failure report lists only the passports that failed, so the second
/// result is not the second passport. A caller matching by index would pin the
/// error on the wrong product.
#[test]
fn a_result_is_found_by_its_identifier_not_its_position() {
    let receipt = SubmissionReceipt {
        correlation_id: "corr-1".into(),
        outcome: SubmissionOutcome::Failure,
        results: vec![PassportResult {
            outcome: Some(PassportOutcome::Rejected),
            error_code: Some("INVALID_UPI_URL".into()),
            ..result("https://id.example.com/01/09506000134352/21/B")
        }],
    };

    let failed = receipt
        .result_for("https://id.example.com/01/09506000134352/21/B")
        .expect("the failed passport is reported");
    assert_eq!(failed.outcome, Some(PassportOutcome::Rejected));
    assert_eq!(failed.error_code.as_deref(), Some("INVALID_UPI_URL"));
    assert!(failed.registration_identifier.is_none());

    // The passport that did nothing wrong is not in the report, and that is
    // not a verdict on it: it was refused with the rest.
    assert!(
        receipt
            .result_for("https://id.example.com/01/09506000134352/21/A")
            .is_none()
    );
}

#[test]
fn a_successful_receipt_carries_one_identifier_per_passport() {
    let receipt = SubmissionReceipt {
        correlation_id: "corr-2".into(),
        outcome: SubmissionOutcome::Success,
        results: vec![
            PassportResult {
                outcome: Some(PassportOutcome::Accepted),
                registration_identifier: Some("URI-1".into()),
                ..result("https://id.example.com/a")
            },
            PassportResult {
                outcome: Some(PassportOutcome::Accepted),
                registration_identifier: Some("URI-2".into()),
                ..result("https://id.example.com/b")
            },
        ],
    };

    let json = serde_json::to_value(&receipt).unwrap();
    assert_eq!(
        json["results"][1],
        serde_json::json!({
            "uniqueProductIdentifier": "https://id.example.com/b",
            "productGroup": "BATTERIES",
            "outcome": "ACCEPTED",
            "registrationIdentifier": "URI-2",
        })
    );
    let back: SubmissionReceipt = serde_json::from_value(json).unwrap();
    assert_eq!(
        back.result_for("https://id.example.com/b")
            .and_then(|r| r.registration_identifier.as_deref()),
        Some("URI-2")
    );
}

/// 🚨 Both of the client's spellings of a finished submission are finished.
///
/// A task's `status` says `SUCCESS` / `FAILURE`; its `outcomeResult` says
/// `SUCCEEDED` / `FAILED`. Read as unknown, either would leave a finished
/// submission polled for ever, since an unknown outcome is not terminal.
#[test]
fn both_spellings_of_a_finished_submission_are_terminal() {
    for (wire, expected) in [
        ("SUCCESS", SubmissionOutcome::Success),
        ("SUCCEEDED", SubmissionOutcome::Success),
        ("FAILURE", SubmissionOutcome::Failure),
        ("FAILED", SubmissionOutcome::Failure),
    ] {
        let parsed: SubmissionOutcome = serde_json::from_value(wire.into()).unwrap();
        assert_eq!(parsed, expected, "{wire}");
        assert!(parsed.is_terminal(), "{wire} must end polling");
    }
    // Written back in the `status` spelling, whichever was read.
    let read: SubmissionOutcome = serde_json::from_value("SUCCEEDED".into()).unwrap();
    assert_eq!(serde_json::to_value(&read).unwrap(), "SUCCESS");
}

/// One row of a registration task's `itemDetails`, in the shape the client
/// reads, becomes a result with nothing lost.
#[test]
fn a_task_item_reads_as_a_passport_result() {
    let row: PassportResult = serde_json::from_value(serde_json::json!({
        "uniqueProductIdentifier": "https://id.example.com/01/09506000134352/21/B",
        "productGroup": "BATTERIES",
        "outcome": "REJECTED",
        "errorCode": "NOT_RESOLVABLE_UPI",
    }))
    .unwrap();
    assert_eq!(row.outcome, Some(PassportOutcome::Rejected));
    assert_eq!(row.error_code.as_deref(), Some("NOT_RESOLVABLE_UPI"));

    // An outcome outside the two is carried, not refused.
    let odd: PassportOutcome = serde_json::from_value("DEFERRED".into()).unwrap();
    assert_eq!(odd, PassportOutcome::Unrecognised("DEFERRED".into()));
    assert_eq!(serde_json::to_value(&odd).unwrap(), "DEFERRED");
}

/// The observed codes are the registry's spelling, each once — a stand-in
/// emitting from this list cannot drift into a near-miss of one.
#[test]
fn the_observed_error_codes_are_distinct_screaming_snake_case() {
    let mut seen = std::collections::BTreeSet::new();
    for code in REGISTRATION_ERROR_CODES {
        assert!(seen.insert(*code), "{code} is listed twice");
        assert!(
            code.chars().all(|c| c.is_ascii_uppercase() || c == '_'),
            "{code} is not in the registry's spelling"
        );
    }
    assert_eq!(REGISTRATION_ERROR_CODES.len(), 22);
}
