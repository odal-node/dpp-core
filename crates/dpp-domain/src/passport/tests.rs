//! Serde round-trips, the typed product-group payload, and the status machine.

use super::*;
use crate::product_group::{CarbonFootprint, ProductGroup, ProductGroupData, RepairabilityScore};
use crate::status::PassportStatus;

pub(crate) fn make_passport() -> Passport {
    Passport {
        id: PassportId(uuid::Uuid::nil()),
        batch_id: Some("BATCH-001".to_owned()),
        serial_number: None,
        product_name: "Eco Widget".to_owned(),
        product_group: ProductGroup::Electronics,
        manufacturer: ManufacturerInfo {
            name: "ACME Corp".to_owned(),
            address: "123 Main St, Berlin, DE".to_owned(),
            country: None,
            did_web_url: Some("https://acme.example.com/.well-known/did.json".to_owned()),
            registered_trade_name: None,
            electronic_address: None,
        },
        materials: vec![MaterialEntry {
            name: "Recycled Aluminium".to_owned(),
            weight_kg: 0.5,
            recycled_pct: Some(80.0),
            country_of_origin: Some("DE".to_owned()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(2.5)),
        repairability_score: Some(RepairabilityScore::from_scalar(7.5)),
        ..crate::test_support::sample_passport()
    }
}

#[test]
fn passport_serde_round_trip() {
    let passport = make_passport();
    let json = serde_json::to_string(&passport).expect("serialise");
    let back: Passport = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(passport.id, back.id);
    assert_eq!(passport.product_name, back.product_name);
    assert_eq!(passport.status, back.status);
    assert_eq!(passport.schema_version, back.schema_version);
}

#[test]
fn passport_carries_typed_product_group() {
    let json = serde_json::to_value(make_passport()).expect("serialise");
    assert_eq!(json["productGroup"], "electronics"); // ProductGroup → camelCase
    let back: Passport = serde_json::from_value(json).expect("deserialise");
    assert_eq!(back.product_group, ProductGroup::Electronics);
}

#[test]
fn product_group_data_mismatch_fails_validation() {
    let mut p = make_passport(); // product_group = Electronics
    p.product_group_data = Some(ProductGroupData::Battery(Box::new(
        crate::test_support::sample_battery_data(),
    )));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("product_group must match"), "got: {err}");
}

/// A passport with no commodity code validates: the regulation asks for one only where relevant.
#[test]
fn missing_commodity_code_is_fine() {
    let mut p = make_passport(); // product_group = Electronics
    p.commodity_code = None;
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn passport_json_uses_camel_case() {
    let passport = make_passport();
    let json = serde_json::to_value(&passport).expect("serialise");
    assert!(
        json.get("productName").is_some(),
        "expected camelCase productName"
    );
    assert!(
        json.get("createdAt").is_some(),
        "expected camelCase createdAt"
    );
    assert!(
        json.get("schemaVersion").is_some(),
        "expected camelCase schemaVersion"
    );
}

#[test]
fn passport_status_serialises_published_as_active() {
    let json = serde_json::to_value(PassportStatus::Published).expect("serialise");
    assert_eq!(json.as_str().unwrap(), "active");
}

#[test]
fn passport_status_deserialises_both_active_and_published() {
    let from_active: PassportStatus = serde_json::from_str("\"active\"").unwrap();
    let from_published: PassportStatus = serde_json::from_str("\"published\"").unwrap();
    assert_eq!(from_active, PassportStatus::Published);
    assert_eq!(from_published, PassportStatus::Published);
}

#[test]
fn transition_draft_to_published_sets_retention_lock() {
    let mut p = make_passport();
    assert_eq!(p.status, PassportStatus::Draft);
    assert!(!p.retention_locked);
    assert!(p.published_at.is_none());

    p.transition_to(PassportStatus::Published).unwrap();

    assert_eq!(p.status, PassportStatus::Published);
    assert!(p.retention_locked);
    assert!(p.published_at.is_some());
}

#[test]
fn transition_invalid_returns_error() {
    let mut p = make_passport();
    // Draft → Suspended is not a valid transition
    let err = p.transition_to(PassportStatus::Suspended);
    assert!(err.is_err());
    // Status should remain unchanged
    assert_eq!(p.status, PassportStatus::Draft);
}

#[test]
fn transition_retired_is_terminal() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Retired).unwrap();
    assert_eq!(p.status, PassportStatus::Retired);

    // Retired → anything is invalid
    assert!(p.transition_to(PassportStatus::Published).is_err());
    assert!(p.transition_to(PassportStatus::Draft).is_err());
}

#[test]
fn transition_re_publish_does_not_overwrite_published_at() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    let first_published = p.published_at;

    p.transition_to(PassportStatus::Suspended).unwrap();
    p.transition_to(PassportStatus::Published).unwrap();

    // published_at should retain the original timestamp
    assert_eq!(p.published_at, first_published);
}

// ── Item-level serial number ─────────────────────────────────────────────────

/// The envelope is additive-only and has no lens, so this is the property that
/// matters most: every passport written before this release must still read.
#[test]
fn a_passport_stored_before_the_serial_existed_reads_with_none() {
    let mut json = serde_json::to_value(make_passport()).expect("serialises");
    json.as_object_mut()
        .expect("object")
        .remove("serialNumber")
        .is_none()
        .then_some(())
        .expect("a minimal passport omits the key, so removing it is a no-op");

    let back: Passport = serde_json::from_value(json).expect("pre-field document reads");
    assert_eq!(back.serial_number, None);
}

/// A passport that declares a commodity code which is not a valid one reads, and
/// writes back as the same JSON. Such a document is accepted when it is stored and
/// signed by someone else when it is fetched, and the envelope has no lens to
/// correct either, so the code is kept as written and is reported where it is used.
#[test]
fn a_passport_declaring_a_malformed_commodity_code_still_reads_and_writes_back_unchanged() {
    for declared in ["not-a-code", " 85076000 ", "8507 60 00"] {
        let mut json = serde_json::to_value(make_passport()).expect("serialises");
        json["commodityCode"] = serde_json::json!(declared);

        let back: Passport =
            serde_json::from_value(json.clone()).expect("a declared code never refuses a read");
        assert_eq!(
            back.commodity_code.as_ref().map(|c| c.as_str()),
            Some(declared)
        );
        assert_eq!(
            serde_json::to_value(&back).unwrap(),
            json,
            "{declared:?} did not write back as it was read"
        );
    }
}

#[test]
fn an_item_serial_round_trips_and_is_omitted_when_unstated() {
    let mut passport = make_passport();
    passport.serial_number = Some("SN-2026-00042".into());

    let json = serde_json::to_value(&passport).expect("serialises");
    assert_eq!(json["serialNumber"], "SN-2026-00042");

    let back: Passport = serde_json::from_value(json).expect("deserialises");
    assert_eq!(back.serial_number.as_deref(), Some("SN-2026-00042"));

    let bare = serde_json::to_value(make_passport()).expect("serialises");
    assert!(
        bare.get("serialNumber").is_none(),
        "an unstated serial is omitted, not nulled: {bare}"
    );
}
