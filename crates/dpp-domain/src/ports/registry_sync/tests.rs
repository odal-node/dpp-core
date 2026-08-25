//! Behaviour of the registration request builder and the granularity mapping.

use super::*;
use crate::{
    passport::{ManufacturerInfo, Passport},
    status::PassportStatus,
};
use chrono::Utc;

fn make_published_passport() -> Passport {
    Passport {
        product_name: "Test".into(),
        manufacturer: ManufacturerInfo {
            name: "ACME".into(),
            address: "Berlin".into(),
            did_web_url: None,
        },
        status: PassportStatus::Published,
        qr_code_url: Some("https://id.odal-node.io/01/09506000134352".into()),
        jws_signature: Some("eyJ0eXAiOiJKV1QifQ.payload.sig".into()),
        published_at: Some(Utc::now()),
        placed_on_market_date: None,
        schema_version: "1.1.0".into(),
        retention_locked: true,
        operator_identifier: Some("did:web:acme.example.com".into()),
        facility: Some(crate::passport::FacilitySnapshot {
            scheme: "national".into(),
            value: "FAC-DE-001".into(),
            name: "Acme Plant".into(),
            country: "DE".into(),
            address: None,
        }),
        ..crate::test_support::sample_passport()
    }
}

/// The operator identity a test registration is filed under.
fn acme() -> RegisteringOperator<'static> {
    RegisteringOperator {
        legal_name: "Acme GmbH",
        country: "DE",
        identifier_scheme: "did",
    }
}

#[test]
fn from_published_passport_maps_all_fields() {
    let passport = make_published_passport();
    let req = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    );

    assert_eq!(req.passport_id, passport.id);
    assert_eq!(req.operator_identifier, "did:web:acme.example.com");
    assert_eq!(req.facility_identifier, "FAC-DE-001");
    // The full facility descriptor is carried, not just the bare identifier.
    assert_eq!(
        req.facility.as_ref().map(|f| f.name.as_str()),
        Some("Acme Plant")
    );
    assert_eq!(
        req.facility.as_ref().map(|f| f.country.as_str()),
        Some("DE")
    );
    assert_eq!(req.product_category, "textile");
    assert_eq!(
        req.data_carrier_uri,
        "https://id.odal-node.io/01/09506000134352"
    );
    assert_eq!(req.schema_version, "1.1.0");
    assert!(req.jws_signature.is_some());
    assert!(req.published_at.is_some());
    assert_eq!(req.country_code, "DE");
    // The operator's legal name comes from operator config, never from the
    // passport's manufacturer block.
    assert_eq!(req.operator_name, "Acme GmbH");
    assert_ne!(
        req.operator_name, passport.manufacturer.name,
        "operator and manufacturer are distinct legal persons"
    );
    assert_eq!(req.granularity, RegistrationGranularity::Item);
}

#[test]
fn from_published_passport_empty_optionals_produce_empty_strings() {
    let mut passport = make_published_passport();
    passport.operator_identifier = None;
    passport.facility = None;
    passport.qr_code_url = None;
    let req = RegistrationRequest::from_published_passport(
        &passport,
        RegisteringOperator {
            legal_name: "",
            country: "",
            identifier_scheme: "",
        },
        RegistrationGranularity::Item,
    );

    assert!(req.operator_identifier.is_empty());
    assert!(req.facility_identifier.is_empty());
    assert!(req.facility.is_none());
    assert!(req.data_carrier_uri.is_empty());
    assert!(req.country_code.is_empty());
}

#[test]
fn registry_status_serde_round_trip() {
    let statuses = vec![
        RegistryStatus::Pending,
        RegistryStatus::Registered,
        RegistryStatus::Rejected,
        RegistryStatus::SuspendedByAuthority,
        RegistryStatus::Deactivated,
    ];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let back: RegistryStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, back);
    }
}

/// Art. 8(4): an item-level registration links the model identifier where a
/// model design exists. It exists for batteries, and the registration must
/// carry it rather than claiming the product has none.
#[test]
fn the_model_identifier_reaches_the_registration() {
    use crate::product_group::ProductGroupData;

    let mut passport = make_published_passport();
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(
        crate::product_group::BatteryData {
            battery_model_id: Some("BM-4815".into()),
            ..crate::test_support::sample_battery_data()
        },
    )));

    let req = RegistrationRequest::from_published_passport(
        &passport,
        acme(),
        RegistrationGranularity::Item,
    );
    assert_eq!(req.model_id.as_deref(), Some("BM-4815"));
}

/// Absent is a substantive answer — the lawful "no model design exists" —
/// and must not be confused with a lookup that was never wired.
#[test]
fn a_product_group_without_a_model_identifier_reports_none() {
    let mut passport = make_published_passport();
    passport.product_group_data = None;
    assert!(
        RegistrationRequest::from_published_passport(
            &passport,
            acme(),
            RegistrationGranularity::Item,
        )
        .model_id
        .is_none()
    );
}
