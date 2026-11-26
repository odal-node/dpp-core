//! Sample identifiers and payloads the registry tests start from.

use super::super::*;
use chrono::Utc;
use uuid::Uuid;

pub(super) fn sample_product_id() -> ProductIdentifier {
    ProductIdentifier {
        scheme: "gtin".into(),
        value: "09506000134352".into(),
        label: Some("Organic Cotton T-Shirt".into()),
    }
}

pub(super) fn sample_item_id() -> ProductItemIdentifier {
    ProductItemIdentifier {
        scheme: "sgtin".into(),
        value: "09506000134352.21.ABC123".into(),
        batch_id: Some("BATCH-2026-Q2-001".into()),
    }
}

pub(super) fn sample_facility_id() -> FacilityIdentifier {
    FacilityIdentifier {
        scheme: "gln".into(),
        value: "4012345000009".into(),
        name: Some("Dhaka Manufacturing Unit 3".into()),
        country: "BD".into(),
        address: Some("123 Industrial Zone, Gazipur".into()),
    }
}

/// An LEI, because a registration carries an identifier with an EN 18219 clause 6
/// route, and a VAT number has none.
pub(super) fn sample_operator_id() -> OperatorIdentifier {
    OperatorIdentifier {
        scheme: "lei".into(),
        value: "529900T8BM49AURSDO55".into(),
        name: "EcoTextile GmbH".into(),
        country: "DE".into(),
        did: Some("did:web:ecotextile.de".into()),
    }
}

pub(crate) fn sample_payload() -> RegistrationPayload {
    RegistrationPayload {
        passport_id: Uuid::nil(),
        product_id: sample_product_id(),
        // Item level: the only level the registry currently accepts, and the
        // one the battery product group is defined at.
        level: RegistrationLevel::new(Granularity::Item).with_model("MODEL-1"),
        item_id: Some(sample_item_id()),
        facility_id: sample_facility_id(),
        operator_id: sample_operator_id(),
        product_group: "textile".into(),
        schema_version: "1.1.0".into(),
        digital_link_url: "https://id.ecotextile.de/01/09506000134352/21/ABC123".into(),
        published_at: Utc::now(),
        jws_signature: Some("eyJhbGciOiJFZERTQSJ9...".into()),
        commodity_code: Some("85076000".into()),
        backup_url: Some("https://backup.example.com/dpp/abc.json".into()),
        service_provider: Some(ServiceProviderReference::named("Example Backup GmbH")),
    }
}
