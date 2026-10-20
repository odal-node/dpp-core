//! What the registry's answers look like as types: a transfer notification's
//! [`TransferReport`] and a lookup's [`SubmissionLookup`].
//!
//! The request builder and the granularity mapping are in `tests`, whose
//! fixtures these share.

use super::tests::{acme, make_published_passport};
use super::*;

/// Only a settled notification stops being sent. `Processing` and an
/// unrecognised state both end in a resend, so a caller that closed on either
/// would record a handover the registry may not have taken.
#[test]
fn a_transfer_report_is_terminal_only_once_settled() {
    assert!(TransferReport::Accepted.is_terminal());
    assert!(TransferReport::AlreadyHeld.is_terminal());
    assert!(
        TransferReport::Refused {
            reason: "no such record".into()
        }
        .is_terminal()
    );
    assert!(!TransferReport::Processing.is_terminal());
    assert!(
        !TransferReport::Unrecognised {
            state: "UNDER_REVIEW".into()
        }
        .is_terminal()
    );
}

/// A caller stores both between passes, and the wire names are part of that.
#[test]
fn the_transfer_report_and_the_lookup_round_trip() {
    for (report, wire) in [
        (
            TransferReport::Accepted,
            serde_json::json!({"outcome": "accepted"}),
        ),
        (
            TransferReport::AlreadyHeld,
            serde_json::json!({"outcome": "alreadyHeld"}),
        ),
        (
            TransferReport::Processing,
            serde_json::json!({"outcome": "processing"}),
        ),
        (
            TransferReport::Refused {
                reason: "no such record".into(),
            },
            serde_json::json!({"outcome": "refused", "reason": "no such record"}),
        ),
        (
            TransferReport::Unrecognised {
                state: "UNDER_REVIEW".into(),
            },
            serde_json::json!({"outcome": "unrecognised", "state": "UNDER_REVIEW"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&report).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<TransferReport>(wire).unwrap(),
            report
        );
    }

    for (lookup, wire) in [
        (
            SubmissionLookup::Found {
                correlation_id: "corr-1".into(),
                report: SubmissionReport::Processing,
            },
            serde_json::json!({
                "outcome": "found",
                "correlationId": "corr-1",
                "report": {"outcome": "processing"},
            }),
        ),
        (
            SubmissionLookup::NotFound { exhaustive: false },
            serde_json::json!({"outcome": "notFound", "exhaustive": false}),
        ),
        (
            SubmissionLookup::Ambiguous {
                candidates: vec!["corr-1".into(), "corr-2".into()],
            },
            serde_json::json!({
                "outcome": "ambiguous",
                "candidates": ["corr-1", "corr-2"],
            }),
        ),
        (
            SubmissionLookup::Unsupported,
            serde_json::json!({"outcome": "unsupported"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&lookup).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<SubmissionLookup>(wire).unwrap(),
            lookup
        );
    }
}

/// Only a handle shown to be the passport's may be recorded. Every other answer
/// leaves it unrecorded, including the ones that are not refusals: a submission
/// still being checked, and an adapter that cannot read one.
#[test]
fn only_a_confirmed_handle_is_confirmed() {
    assert!(
        SubmissionConfirmation::Confirmed {
            report: SubmissionReport::Processing
        }
        .is_confirmed()
    );
    for not_confirmed in [
        SubmissionConfirmation::Mismatch {
            reported: vec!["https://example.test/01/1".into()],
        },
        SubmissionConfirmation::Mismatch { reported: vec![] },
        SubmissionConfirmation::StillChecking,
        SubmissionConfirmation::Unlisted { exhaustive: true },
        SubmissionConfirmation::Unlisted { exhaustive: false },
        SubmissionConfirmation::Unsupported,
    ] {
        assert!(!not_confirmed.is_confirmed(), "{not_confirmed:?}");
    }
}

/// A caller stores the answer beside the handle it checked, and the wire names
/// are part of that.
#[test]
fn a_confirmation_round_trips() {
    for (confirmation, wire) in [
        (
            SubmissionConfirmation::Confirmed {
                report: SubmissionReport::Registered {
                    registration_id: "reg-1".into(),
                },
            },
            serde_json::json!({
                "outcome": "confirmed",
                "report": {"outcome": "registered", "registrationId": "reg-1"},
            }),
        ),
        (
            SubmissionConfirmation::Mismatch {
                reported: vec!["https://example.test/01/1".into()],
            },
            serde_json::json!({
                "outcome": "mismatch",
                "reported": ["https://example.test/01/1"],
            }),
        ),
        (
            SubmissionConfirmation::StillChecking,
            serde_json::json!({"outcome": "stillChecking"}),
        ),
        (
            SubmissionConfirmation::Unlisted { exhaustive: true },
            serde_json::json!({"outcome": "unlisted", "exhaustive": true}),
        ),
        (
            SubmissionConfirmation::Unsupported,
            serde_json::json!({"outcome": "unsupported"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&confirmation).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<SubmissionConfirmation>(wire).unwrap(),
            confirmation
        );
    }
}

/// An adapter written before `find_submission` existed implements none of it,
/// and has to compile and answer honestly: it cannot look a submission up, which
/// is not the same as the registry holding none. The same goes for confirming a
/// handle, which is not the same as the handle being wrong.
#[tokio::test]
async fn an_adapter_that_cannot_look_a_submission_up_says_so() {
    struct Bare;

    #[async_trait::async_trait]
    impl RegistrySyncPort for Bare {
        async fn register(
            &self,
            _request: RegistrationRequest,
        ) -> Result<SubmissionAcknowledgement, crate::error::DppError> {
            unreachable!("not exercised")
        }
        async fn check_submission(
            &self,
            _correlation_id: &str,
        ) -> Result<SubmissionReport, crate::error::DppError> {
            unreachable!("not exercised")
        }
        async fn check_status(
            &self,
            _registration_id: &str,
        ) -> Result<RegistryRecord, crate::error::DppError> {
            unreachable!("not exercised")
        }
        async fn notify_transfer(
            &self,
            _record: &crate::transfer::TransferRecord,
            _registration_id: &str,
        ) -> Result<TransferReport, crate::error::DppError> {
            unreachable!("not exercised")
        }
    }

    let request = RegistrationRequest::from_published_passport(
        &make_published_passport(),
        acme(),
        RegistrationGranularity::Item,
    )
    .expect("the fixture passport is complete");

    assert_eq!(
        Bare.find_submission(&request).await.unwrap(),
        SubmissionLookup::Unsupported
    );
    assert_eq!(
        Bare.confirm_submission(&request, "corr-1").await.unwrap(),
        SubmissionConfirmation::Unsupported
    );
}
