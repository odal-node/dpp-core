//! Identifier and country validation.

use super::super::*;

#[test]
fn valid_gtin_product_identifier_passes() {
    let id = ProductIdentifier {
        scheme: "gtin".into(),
        value: "09506000134352".into(),
        label: None,
    };
    assert!(id.validate().is_ok());
}

#[test]
fn invalid_gtin_product_identifier_fails() {
    let id = ProductIdentifier {
        scheme: "gtin".into(),
        value: "12345678901234".into(), // bad check digit
        label: None,
    };
    assert!(matches!(
        id.validate(),
        Err(RegistryValidationError::InvalidGtin { .. })
    ));
}

#[test]
fn non_gtin_scheme_skips_checksum_validation() {
    let id = ProductIdentifier {
        scheme: "passport_id".into(),
        value: "not-a-gtin-at-all".into(),
        label: None,
    };
    assert!(id.validate().is_ok());
}

#[test]
fn valid_iso_country_passes() {
    let fac = FacilityIdentifier {
        scheme: "gln".into(),
        value: "4012345000009".into(),
        name: None,
        country: "DE".into(),
        address: None,
    };
    assert!(fac.validate().is_ok());
}

#[test]
fn empty_country_rejected() {
    // `country` is a mandatory Annex III field — an empty value is a missing
    // required identifier, not an acceptable "unknown".
    let fac = FacilityIdentifier {
        scheme: "national".into(),
        value: "FAC-001".into(),
        name: None,
        country: String::new(),
        address: None,
    };
    assert!(matches!(
        fac.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));
}

#[test]
fn gln_facility_bad_check_digit_rejected() {
    let fac = FacilityIdentifier {
        scheme: "gln".into(),
        value: "4000001000002".into(), // shape-valid but wrong GS1 check digit
        name: None,
        country: "DE".into(),
        address: None,
    };
    assert!(matches!(
        fac.validate(),
        Err(RegistryValidationError::InvalidGln { .. })
    ));
}

#[test]
fn lei_operator_checksum_validated() {
    let valid = OperatorIdentifier {
        scheme: "lei".into(),
        value: "5493001KJTIIGC8Y1R12".into(), // valid ISO 7064 MOD 97-10
        name: "Example AG".into(),
        country: "DE".into(),
        did: None,
    };
    assert!(valid.validate().is_ok());

    let bad = OperatorIdentifier {
        value: "969500GU3KE7GR9NDV41".into(), // wrong check digits
        ..valid
    };
    assert!(matches!(
        bad.validate(),
        Err(RegistryValidationError::InvalidOperatorId { .. })
    ));
}

#[test]
fn duns_and_eori_structure_validated() {
    let duns_ok = OperatorIdentifier {
        scheme: "duns".into(),
        value: "150483782".into(),
        name: "X".into(),
        country: "US".into(),
        did: None,
    };
    assert!(duns_ok.validate().is_ok());

    let duns_bad = OperatorIdentifier {
        value: "15048378".into(), // 8 digits
        ..duns_ok.clone()
    };
    assert!(duns_bad.validate().is_err());

    let eori_ok = OperatorIdentifier {
        scheme: "eori".into(),
        value: "DE1234567890".into(),
        ..duns_ok.clone()
    };
    assert!(eori_ok.validate().is_ok());

    let eori_bad = OperatorIdentifier {
        scheme: "eori".into(),
        value: "1234567890".into(), // missing 2-letter country prefix
        ..duns_ok
    };
    assert!(eori_bad.validate().is_err());
}

#[test]
fn unknown_operator_scheme_not_structurally_verified() {
    let op = OperatorIdentifier {
        scheme: "custom".into(),
        value: "anything-goes".into(),
        name: "X".into(),
        country: "DE".into(),
        did: None,
    };
    assert!(op.validate().is_ok());
}

#[test]
fn eu_pseudo_code_rejected() {
    let op = OperatorIdentifier {
        scheme: "did".into(),
        value: "did:web:acme.example.com".into(),
        name: "ACME".into(),
        country: "EU".into(),
        did: None,
    };
    assert!(matches!(
        op.validate(),
        Err(RegistryValidationError::InvalidCountryCode { .. })
    ));
}

#[test]
fn lowercase_country_rejected() {
    let op = OperatorIdentifier {
        scheme: "vat".into(),
        value: "DE123456789".into(),
        name: "Test".into(),
        country: "de".into(),
        did: None,
    };
    assert!(matches!(
        op.validate(),
        Err(RegistryValidationError::InvalidCountryCode { .. })
    ));
}
