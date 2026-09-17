//! What a submission admits, what it refuses, and that it refuses as a whole.

use super::{MAX_PASSPORTS_PER_SUBMISSION, RegistrationSubmission};
use crate::error::RegistryValidationError;

/// A valid payload, cloned as many times as a case needs.
fn payload() -> crate::RegistrationPayload {
    crate::tests::sample_payload()
}

/// One that fails `RegistrationPayload::validate` — an item-level registration
/// with no item identifier, which Art. 8(1) requires.
fn invalid_payload() -> crate::RegistrationPayload {
    crate::RegistrationPayload {
        item_id: None,
        ..payload()
    }
}

#[test]
fn a_single_passport_submission_is_the_common_case() {
    let submission = RegistrationSubmission::single(payload());
    assert_eq!(submission.len(), 1);
    assert!(!submission.is_empty());
    assert!(submission.validate().is_ok());
}

/// The bounds, at both ends.
#[test]
fn the_submission_bounds_are_enforced() {
    assert!(matches!(
        RegistrationSubmission::new(Vec::new()),
        Err(RegistryValidationError::EmptySubmission)
    ));

    // Exactly at the cap is accepted — an off-by-one here would refuse a
    // submission the registry takes, which is the more expensive direction.
    let at_cap = vec![payload(); MAX_PASSPORTS_PER_SUBMISSION];
    assert_eq!(
        RegistrationSubmission::new(at_cap).map(|s| s.len()).ok(),
        Some(MAX_PASSPORTS_PER_SUBMISSION)
    );

    let over = vec![payload(); MAX_PASSPORTS_PER_SUBMISSION + 1];
    assert!(matches!(
        RegistrationSubmission::new(over),
        Err(RegistryValidationError::SubmissionTooLarge { count, max })
            if count == MAX_PASSPORTS_PER_SUBMISSION + 1 && max == MAX_PASSPORTS_PER_SUBMISSION
    ));
}

/// 🚨 The rule this type exists for: one bad passport refuses **all** of them.
///
/// 👁️ User Guide v1.02 — *"When submitting multiple DPPs, if a single DPP has an
/// error, all the DPPs in the same submission will be rejected."*
///
/// The failing passport is last on purpose. A validator that stopped early, or
/// one that only ever checked the first element, would pass a test putting the
/// bad passport at index 0 — and the registry's rule is about the whole set.
#[test]
fn one_invalid_passport_refuses_the_whole_submission() {
    let mut passports = vec![payload(), payload(), payload()];
    passports.push(invalid_payload());
    let submission = RegistrationSubmission::new(passports).expect("within bounds");

    assert_eq!(submission.len(), 4, "the submission is well-formed");
    assert!(
        matches!(
            submission.validate(),
            Err(RegistryValidationError::SubmissionPassportInvalid { index, .. }) if index == 3
        ),
        "the index of the offending passport must be reported, got {:?}",
        submission.validate()
    );
}

/// The index is what makes an all-or-nothing refusal actionable, so it is the
/// offender's — not the first, not the last.
#[test]
fn the_reported_index_is_the_offending_passport() {
    let passports = vec![payload(), invalid_payload(), payload()];
    let submission = RegistrationSubmission::new(passports).expect("within bounds");

    let Err(RegistryValidationError::SubmissionPassportInvalid { index, source }) =
        submission.validate()
    else {
        panic!("an invalid passport must refuse the submission");
    };
    assert_eq!(index, 1);
    // …and the passport's own error is carried, not flattened into a message.
    assert!(matches!(
        *source,
        RegistryValidationError::MissingRequiredField(ref f) if f == "itemId"
    ));
}

/// The bound is an invariant on the way back too: a stored or in-flight
/// submission the registry would refuse must not deserialise into a value that
/// looks well-formed.
#[test]
fn deserialisation_is_held_to_the_same_bounds() {
    let over: Vec<serde_json::Value> = (0..=MAX_PASSPORTS_PER_SUBMISSION)
        .map(|_| serde_json::to_value(payload()).unwrap())
        .collect();
    assert!(
        serde_json::from_value::<RegistrationSubmission>(serde_json::json!(over)).is_err(),
        "a submission past the cap was accepted through serde"
    );

    assert!(
        serde_json::from_value::<RegistrationSubmission>(serde_json::json!([])).is_err(),
        "an empty submission was accepted through serde"
    );
}

/// It travels as a bare array — the wrapper is ours and must not reach the wire.
#[test]
fn a_submission_serialises_as_its_passports() {
    let submission = RegistrationSubmission::single(payload());
    let json = serde_json::to_value(&submission).unwrap();

    assert!(json.is_array(), "got {json}");
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json[0]["productGroup"], "textile");

    let back: RegistrationSubmission = serde_json::from_value(json).unwrap();
    assert_eq!(back, submission);
}
