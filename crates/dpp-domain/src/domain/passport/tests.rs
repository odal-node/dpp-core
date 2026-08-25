//! Serde round-trips, the typed product-group payload, and the status machine.

use super::*;
use crate::domain::product_group::{
    CarbonFootprint, ProductGroup, ProductGroupData, RepairabilityScore, UnsoldGoodsReport,
};
use crate::domain::status::PassportStatus;

pub(super) fn make_passport() -> Passport {
    Passport {
        id: PassportId(uuid::Uuid::nil()),
        batch_id: Some("BATCH-001".to_owned()),
        product_name: "Eco Widget".to_owned(),
        product_group: ProductGroup::Electronics,
        manufacturer: ManufacturerInfo {
            name: "ACME Corp".to_owned(),
            address: "123 Main St, Berlin, DE".to_owned(),
            did_web_url: Some("https://acme.example.com/.well-known/did.json".to_owned()),
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

#[test]
fn missing_commodity_code_is_fine_outside_unsold_goods() {
    let mut p = make_passport(); // product_group = Electronics
    p.commodity_code = None;
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

fn unsold_goods_report() -> UnsoldGoodsReport {
    crate::test_support::sample_unsold_goods_report()
}

#[test]
fn an_unsold_goods_disclosure_with_lines_validates() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(unsold_goods_report()));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

/// Art. 24's disclosure duty reaches discarded unsold **consumer products**
/// generally; Art. 25's destruction ban reaches Annex VII's apparel and
/// footwear. An earlier check required Annex VII scope here and so rejected
/// every lawful disclosure outside those two, which is most of Annex II's 45
/// headings.
#[test]
fn a_disclosure_outside_annex_vii_scope_is_not_rejected() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    let mut report = unsold_goods_report();
    // Refrigerators — Annex II heading 8418, nowhere near Annex VII.
    report.lines[0].cn_categories =
        vec![crate::domain::product_group::CnCategory::parse("8418").expect("valid heading")];
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(report));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

/// The envelope's `commodity_code` describes a product; a disclosure has none,
/// so it must not be required here.
#[test]
fn an_unsold_goods_disclosure_needs_no_envelope_commodity_code() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(unsold_goods_report()));
    p.commodity_code = None;
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn an_unsold_goods_disclosure_with_no_lines_fails() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    let mut report = unsold_goods_report();
    report.lines.clear();
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(report));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("at least one product line"), "got: {err}");
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
fn transition_archived_is_terminal() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Archived).unwrap();
    assert_eq!(p.status, PassportStatus::Archived);

    // Archived → anything is invalid
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
