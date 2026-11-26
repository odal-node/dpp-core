//! The verification outcomes a caller must tell apart: a revocation status that
//! cannot be answered is not a revocation, and a `validFrom` still in the future
//! is not an expiry.
//!
//! The claims check and the revocation check are shared by both verifiers, so
//! every case here runs through `verify_credential_with_revocation` and through
//! `verify_credential_with_revocation_and_trust`, and both must answer the same way.

use chrono::{DateTime, Duration, Utc};

use crate::status_list::StatusList;

use super::super::super::*;
use super::super::trust::{credential_with_status_index, sample_credential};

/// Runs both revocation verifiers on one credential, trusting its issuer, and
/// asserts each returns `expected`.
fn assert_both_answer(
    cred: &DppAccessCredential,
    now: DateTime<Utc>,
    list: Option<&StatusList>,
    expected: VerificationResult,
) {
    let trusted = StaticTrustedIssuers::single(cred.issuer.clone());
    assert_eq!(
        verify_credential_with_revocation(cred, None, now, list),
        expected,
        "verify_credential_with_revocation"
    );
    assert_eq!(
        verify_credential_with_revocation_and_trust(cred, None, None, now, list, &trusted),
        expected,
        "verify_credential_with_revocation_and_trust"
    );
}

/// The `Valid` result for `sample_credential`, which grants a legitimate interest.
fn valid_result(cred: &DppAccessCredential) -> VerificationResult {
    VerificationResult::Valid {
        audience: Audience::LegitimateInterest,
        role: CredentialRole::AuthorisedRepairer,
        holder_did: cred.credential_subject.id.clone(),
    }
}

/// Bit 5 set. `credential_with_status_index("5")` reads this bit.
fn list_with_bit_5_set() -> StatusList {
    StatusList::from_bitstring(vec![0b0000_0100])
}

#[test]
fn declared_status_with_no_list_is_indeterminate_not_revoked() {
    let cred = credential_with_status_index("5");
    assert_both_answer(
        &cred,
        Utc::now(),
        None,
        VerificationResult::RevocationIndeterminate,
    );
}

#[test]
fn index_out_of_range_is_indeterminate_not_revoked() {
    // The list holds eight entries, so index 999 is outside it.
    let cred = credential_with_status_index("999");
    let list = list_with_bit_5_set();
    assert_both_answer(
        &cred,
        Utc::now(),
        Some(&list),
        VerificationResult::RevocationIndeterminate,
    );
}

#[test]
fn suspension_purpose_is_indeterminate_even_with_the_bit_set() {
    let mut cred = credential_with_status_index("5");
    cred.credential_status
        .as_mut()
        .expect("the fixture declares a status")
        .status_purpose = "suspension".into();
    let list = list_with_bit_5_set();
    assert_both_answer(
        &cred,
        Utc::now(),
        Some(&list),
        VerificationResult::RevocationIndeterminate,
    );
}

#[test]
fn set_bit_is_revoked() {
    let cred = credential_with_status_index("5");
    let list = list_with_bit_5_set();
    assert_both_answer(&cred, Utc::now(), Some(&list), VerificationResult::Revoked);
}

#[test]
fn no_credential_status_is_valid_whatever_list_is_given() {
    let cred = sample_credential();
    let list = list_with_bit_5_set();
    assert_both_answer(&cred, Utc::now(), None, valid_result(&cred));
    assert_both_answer(&cred, Utc::now(), Some(&list), valid_result(&cred));
}

#[test]
fn valid_from_after_now_is_not_yet_valid_carrying_that_valid_from() {
    let mut cred = sample_credential();
    let now = Utc::now();
    cred.valid_from = now + Duration::hours(1);
    let expected = VerificationResult::NotYetValid {
        valid_from: cred.valid_from,
    };
    assert_both_answer(&cred, now, None, expected);
}

#[test]
fn valid_from_equal_to_now_is_valid_because_the_comparison_is_exact() {
    let mut cred = sample_credential();
    let now = Utc::now();
    cred.valid_from = now;
    assert_both_answer(&cred, now, None, valid_result(&cred));
}

#[test]
fn the_two_new_variants_are_not_valid() {
    assert!(!VerificationResult::RevocationIndeterminate.is_valid());
    assert!(
        !VerificationResult::NotYetValid {
            valid_from: Utc::now(),
        }
        .is_valid()
    );
}
