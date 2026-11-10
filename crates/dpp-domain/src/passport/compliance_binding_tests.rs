//! The first publish refuses a compliance determination that is not bound to the
//! `productGroupData` the passport holds, and refuses nothing else about it.

use super::Passport;
use crate::compliance::ComplianceResult;
use crate::error::dpp::DppError;
use crate::product_group::ProductGroupData;
use crate::status::PassportStatus;
use crate::test_support::{sample_passport, sample_textile_data};

/// A textile draft carrying `productGroupData`, so a determination has data to bind to.
fn draft_with_data() -> Passport {
    let mut passport = sample_passport();
    passport.product_group_data = Some(ProductGroupData::Textile(Box::new(sample_textile_data())));
    passport
}

/// Whether a refusal names the compliance determination as the field at fault.
fn refused_on_compliance_result(err: &DppError) -> bool {
    matches!(err, DppError::Validation(errors) if errors.errors.iter().any(|e| e.field == "/complianceResult"))
}

#[test]
fn a_determination_bound_to_its_data_publishes() {
    let mut passport = draft_with_data();
    passport
        .attach_compliance_result(ComplianceResult::default())
        .unwrap();

    passport.transition_to(PassportStatus::Published).unwrap();
    assert_eq!(passport.status, PassportStatus::Published);
}

#[test]
fn a_determination_from_older_data_is_refused_until_reassessed() {
    let mut passport = draft_with_data();
    passport
        .attach_compliance_result(ComplianceResult::default())
        .unwrap();

    let mut edited = sample_textile_data();
    edited.country_of_origin = "DE".into();
    passport.product_group_data = Some(ProductGroupData::Textile(Box::new(edited)));

    let err = passport
        .transition_to(PassportStatus::Published)
        .unwrap_err();
    assert!(refused_on_compliance_result(&err));
    assert_eq!(passport.status, PassportStatus::Draft);

    passport
        .attach_compliance_result(ComplianceResult::default())
        .unwrap();
    passport.transition_to(PassportStatus::Published).unwrap();
    assert_eq!(passport.status, PassportStatus::Published);
}

#[test]
fn a_determination_that_was_never_bound_is_refused() {
    let mut passport = draft_with_data();
    passport.compliance_result = Some(ComplianceResult::default());

    let err = passport
        .transition_to(PassportStatus::Published)
        .unwrap_err();
    assert!(refused_on_compliance_result(&err));
}

#[test]
fn a_passport_with_no_determination_publishes_as_before() {
    let mut passport = draft_with_data();
    assert!(passport.compliance_result.is_none());

    passport.transition_to(PassportStatus::Published).unwrap();
    assert_eq!(passport.status, PassportStatus::Published);
}
