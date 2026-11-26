//! Serde round-trips of the registry types.

use super::super::*;
use super::common::*;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn registration_payload_round_trip() {
    let payload = sample_payload();
    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["productGroup"], "textile");
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
        submission: RegistrationSubmission::single(sample_payload()),
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
        node_acceptance_attestation: Some("sig_to...".into()),
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
