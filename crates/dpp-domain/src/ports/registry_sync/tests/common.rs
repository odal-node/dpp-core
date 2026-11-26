//! A published passport and a registering operator the registry tests share.

use super::super::*;
use crate::{
    passport::{ManufacturerInfo, Passport},
    status::PassportStatus,
};
use chrono::Utc;

pub(super) fn make_published_passport() -> Passport {
    Passport {
        product_name: "Test".into(),
        manufacturer: ManufacturerInfo {
            name: "ACME".into(),
            address: "Berlin".into(),
            country: None,
            did_web_url: None,
            registered_trade_name: None,
            electronic_address: None,
        },
        status: PassportStatus::Published,
        qr_code_url: Some("https://id.odal-node.io/01/09506000134352".into()),
        jws_signature: Some("eyJ0eXAiOiJKV1QifQ.payload.sig".into()),
        published_at: Some(Utc::now()),
        placed_on_market_date: None,
        schema_version: "1.1.0".into(),
        retention_locked: true,
        operator_identifier: Some("did:web:acme.example.com".into()),
        product_group_data: Some(crate::product_group::ProductGroupData::Textile(Box::new(
            crate::test_support::sample_textile_data(),
        ))),
        facility: Some(crate::passport::FacilitySnapshot {
            scheme: "national".into(),
            value: "FAC-DE-001".into(),
            name: "Acme Plant".into(),
            country: "DE".into(),
            address: None,
            identifier: None,
        }),
        ..crate::test_support::sample_passport()
    }
}

/// The operator identity a test registration is filed under.
pub(super) fn acme() -> RegisteringOperator<'static> {
    RegisteringOperator {
        legal_name: "Acme GmbH",
        country: "DE",
        identifier_scheme: "did",
    }
}
