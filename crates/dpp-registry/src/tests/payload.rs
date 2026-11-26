//! Validation of a registration payload.

use super::super::*;
use super::common::*;

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
fn empty_item_id_rejected() {
    let mut payload = sample_payload();
    payload.item_id = Some(ProductItemIdentifier {
        scheme: String::new(),
        value: String::new(),
        batch_id: None,
    });
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));
}

#[test]
fn empty_product_group_or_schema_version_rejected() {
    let mut payload = sample_payload();
    payload.product_group = String::new();
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));

    let mut payload = sample_payload();
    payload.schema_version = String::new();
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
    ));
}

#[test]
fn empty_operator_name_rejected() {
    let op = OperatorIdentifier {
        scheme: "vat".into(),
        value: "DE123456789".into(),
        name: String::new(),
        country: "DE".into(),
        did: None,
    };
    assert!(matches!(
        op.validate(),
        Err(RegistryValidationError::MissingRequiredField(_))
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

// ── Registration level (IR (EU) 2026/1778 Art. 8) ───────────────────────────

/// Art. 8(1): an item-level registration must identify the unit it covers.
#[test]
fn item_level_payload_without_an_item_id_rejected() {
    let mut payload = sample_payload();
    payload.item_id = None;
    assert!(matches!(
        payload.validate(),
        Err(RegistryValidationError::MissingRequiredField(f)) if f == "itemId"
    ));
}

/// Above item level the registration covers a group, so naming a single unit
/// contradicts the Art. 8(1) level the registry checks under Art. 8(7)(c).
#[test]
fn model_and_batch_level_payloads_must_not_carry_an_item_id() {
    for granularity in [Granularity::Model, Granularity::Batch] {
        let mut payload = sample_payload();
        payload.level = RegistrationLevel::new(granularity);
        assert!(
            matches!(
                payload.validate(),
                Err(RegistryValidationError::GranularityMismatch {
                    identifier: "itemId",
                    ..
                })
            ),
            "a {granularity} registration must not carry an item identifier"
        );
    }
}

/// A model- or batch-level registration is valid without an item identifier —
/// the levels the registry will accept once further product groups land.
#[test]
fn model_and_batch_level_payloads_validate_without_an_item_id() {
    for granularity in [Granularity::Model, Granularity::Batch] {
        let mut payload = sample_payload();
        payload.level = RegistrationLevel::new(granularity);
        payload.item_id = None;
        assert!(
            payload.validate().is_ok(),
            "a {granularity}-level registration needs no item identifier"
        );
    }
}

/// The level travels on the wire — the registry validates it on submission.
#[test]
fn registration_level_serialises_into_the_payload() {
    let payload = sample_payload();
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["level"]["granularity"], "item");
    assert_eq!(json["level"]["modelId"], "MODEL-1");
    assert!(
        json["level"].get("batchId").is_none(),
        "an unlinked batch must be absent, not null"
    );
}

#[test]
fn granularity_mismatch_display_message() {
    let err = RegistryValidationError::GranularityMismatch {
        granularity: "model",
        identifier: "batchId",
    };
    assert_eq!(
        err.to_string(),
        "a 'model'-level registration must not carry a 'batchId'"
    );
}

// ── Transfer notification validation ────────────────────────────────────────
