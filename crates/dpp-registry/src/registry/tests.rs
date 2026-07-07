//! Serde round-trip and B1 identifier-validation tests for the registry types.

use super::*;
use chrono::Utc;
use uuid::Uuid;

fn sample_product_id() -> ProductIdentifier {
    ProductIdentifier {
        scheme: "gtin".into(),
        value: "09506000134352".into(),
        label: Some("Organic Cotton T-Shirt".into()),
    }
}

fn sample_item_id() -> ProductItemIdentifier {
    ProductItemIdentifier {
        scheme: "sgtin".into(),
        value: "09506000134352.21.ABC123".into(),
        batch_id: Some("BATCH-2026-Q2-001".into()),
    }
}

fn sample_facility_id() -> FacilityIdentifier {
    FacilityIdentifier {
        scheme: "gln".into(),
        value: "4012345000009".into(),
        name: Some("Dhaka Manufacturing Unit 3".into()),
        country: "BD".into(),
        address: Some("123 Industrial Zone, Gazipur".into()),
    }
}

fn sample_operator_id() -> OperatorIdentifier {
    OperatorIdentifier {
        scheme: "vat".into(),
        value: "DE123456789".into(),
        name: "EcoTextile GmbH".into(),
        country: "DE".into(),
        did: Some("did:web:ecotextile.de".into()),
    }
}

fn sample_payload() -> RegistrationPayload {
    RegistrationPayload {
        passport_id: Uuid::nil(),
        product_id: sample_product_id(),
        item_id: sample_item_id(),
        facility_id: sample_facility_id(),
        operator_id: sample_operator_id(),
        sector: "textile".into(),
        schema_version: "1.1.0".into(),
        digital_link_url: "https://id.ecotextile.de/01/09506000134352/21/ABC123".into(),
        published_at: Utc::now(),
        jws_signature: Some("eyJhbGciOiJFZERTQSJ9...".into()),
    }
}

#[test]
fn registration_payload_round_trip() {
    let payload = sample_payload();
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["sector"], "textile");
    assert_eq!(json["productId"]["scheme"], "gtin");
    assert_eq!(json["operatorId"]["country"], "DE");
    let back: RegistrationPayload = serde_json::from_value(json).unwrap();
    assert_eq!(payload.passport_id, back.passport_id);
    assert_eq!(payload.product_id, back.product_id);
}

#[test]
fn envelope_round_trip() {
    let envelope = EuRegistryEnvelope {
        api_version: "1.0".into(),
        request_id: Uuid::nil(),
        timestamp: Utc::now(),
        payload: sample_payload(),
    };
    let json = serde_json::to_string(&envelope).unwrap();
    let back: EuRegistryEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(envelope.api_version, back.api_version);
}

#[test]
fn response_with_rejection() {
    let response = EuRegistryResponse {
        registry_id: "EU-REG-2026-00001".into(),
        passport_id: Uuid::nil(),
        status: RegistryStatusCode::Rejected,
        message: Some("Validation failed".into()),
        rejection_reasons: Some(vec![
            "Product identifier scheme 'custom' not recognized".into(),
            "Facility country 'XX' is not a valid ISO 3166-1 code".into(),
        ]),
        updated_at: Utc::now(),
    };
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["status"], "rejected");
    assert_eq!(json["rejectionReasons"].as_array().unwrap().len(), 2);
    let back: EuRegistryResponse = serde_json::from_value(json).unwrap();
    assert_eq!(back.status, RegistryStatusCode::Rejected);
}

#[test]
fn transfer_notification_round_trip() {
    let notif = TransferNotification {
        passport_id: Uuid::nil(),
        registry_id: "EU-REG-2026-00001".into(),
        from_operator: sample_operator_id(),
        to_operator: OperatorIdentifier {
            scheme: "vat".into(),
            value: "FR987654321".into(),
            name: "ModeVerte SARL".into(),
            country: "FR".into(),
            did: Some("did:web:modeverte.fr".into()),
        },
        reason: "sale".into(),
        transferred_at: Utc::now(),
        from_signature: Some("sig_from...".into()),
        to_signature: Some("sig_to...".into()),
    };
    let json = serde_json::to_value(&notif).unwrap();
    assert_eq!(json["reason"], "sale");
    assert_eq!(json["toOperator"]["name"], "ModeVerte SARL");
    let back: TransferNotification = serde_json::from_value(json).unwrap();
    assert_eq!(notif.registry_id, back.registry_id);
}

#[test]
fn error_display() {
    let err = EuRegistryError {
        kind: EuRegistryErrorKind::RegistrationRejected,
        message: "missing facility identifier".into(),
        status_code: Some(422),
        registry_error_code: Some("ERR_MISSING_FACILITY".into()),
    };
    let display = format!("{err}");
    assert!(display.contains("RegistrationRejected"));
    assert!(display.contains("missing facility identifier"));
}

#[test]
fn sandbox_endpoint() {
    let ep = RegistryEndpoint::sandbox();
    assert_eq!(ep.authority, RegistryAuthority::EuSandbox);
    assert!(!ep.mtls_required);
    assert!(ep.base_url.contains("sandbox"));
}

#[test]
fn production_endpoint() {
    let ep = RegistryEndpoint::production();
    assert_eq!(ep.authority, RegistryAuthority::EuCentral);
    assert!(ep.mtls_required);
}

#[test]
fn status_response_round_trip() {
    let status = StatusResponse {
        registry_id: "EU-REG-2026-00001".into(),
        status: RegistryStatusCode::Registered,
        updated_at: Utc::now(),
        message: None,
    };
    let json = serde_json::to_string(&status).unwrap();
    let back: StatusResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(back.status, RegistryStatusCode::Registered);
}

// ── B1 validation tests ─────────────────────────────────────────────────

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
fn empty_country_passes_as_unknown() {
    let fac = FacilityIdentifier {
        scheme: "national".into(),
        value: "FAC-001".into(),
        name: None,
        country: String::new(),
        address: None,
    };
    assert!(fac.validate().is_ok());
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

#[test]
fn valid_payload_passes_validation() {
    assert!(sample_payload().validate().is_ok());
}

#[test]
fn payload_with_empty_digital_link_fails() {
    let mut payload = sample_payload();
    payload.digital_link_url = String::new();
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));
}

#[test]
fn payload_with_invalid_gtin_fails() {
    let mut payload = sample_payload();
    payload.product_id.value = "99999999999999".into(); // bad check digit
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::InvalidGtin { .. })
    ));
}

#[test]
fn validation_error_display_messages() {
    let gtin = RegistryValidationError::InvalidGtin {
        value: "123".into(),
        reason: "too short".into(),
    };
    assert_eq!(gtin.to_string(), "invalid GTIN '123': too short");

    let country = RegistryValidationError::InvalidCountryCode { code: "EU".into() };
    assert!(country.to_string().starts_with("invalid country code 'EU'"));

    let missing = RegistryValidationError::MissingRequiredField("passportId".into());
    assert_eq!(missing.to_string(), "required field 'passportId' is empty");

    // Error trait object is usable (covers the std::error::Error impl).
    let boxed: Box<dyn std::error::Error> = Box::new(gtin);
    assert!(!boxed.to_string().is_empty());
}
