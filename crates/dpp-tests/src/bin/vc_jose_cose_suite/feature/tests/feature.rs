//! Tests of `feature`.

use super::super::*;

#[test]
fn all_six_features_are_implemented() {
    for (name, feature) in [
        ("credential_jose", Feature::CredentialJose),
        ("presentation_jose", Feature::PresentationJose),
        ("credential_sdjwt", Feature::CredentialSdJwt),
        ("presentation_sdjwt", Feature::PresentationSdJwt),
        ("credential_cose", Feature::CredentialCose),
        ("presentation_cose", Feature::PresentationCose),
    ] {
        assert_eq!(Feature::parse(name), Ok(feature), "{name}");
    }
}

#[test]
fn only_the_sd_jwt_features_take_paths() {
    assert!(Feature::CredentialSdJwt.is_sd_jwt());
    assert!(Feature::PresentationSdJwt.is_sd_jwt());
    assert!(!Feature::CredentialJose.is_sd_jwt());
    assert!(!Feature::PresentationJose.is_sd_jwt());
    assert!(!Feature::CredentialCose.is_sd_jwt());
    assert!(!Feature::PresentationCose.is_sd_jwt());
}

#[test]
fn only_the_cose_features_read_and_write_base64() {
    assert!(Feature::CredentialCose.is_cose());
    assert!(Feature::PresentationCose.is_cose());
    assert!(!Feature::CredentialJose.is_cose());
    assert!(!Feature::CredentialSdJwt.is_cose());
}

#[test]
fn a_feature_the_suite_does_not_have_is_refused_by_name() {
    let refusal = Feature::parse("credential_jwt").expect_err("unknown");
    assert!(refusal.contains("credential_jwt"), "{refusal}");
    assert!(Feature::parse("").is_err());
}
