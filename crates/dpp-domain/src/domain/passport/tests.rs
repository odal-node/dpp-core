//! Serde round-trip, state-machine, validation, and redaction tests for `Passport`.

use super::*;
use crate::catalog::ProductGroupCatalog;
use crate::domain::error::DppError;
use crate::domain::identity::Audience;
use crate::domain::product_group::{
    BatteryChemistry, BatteryData, CarbonFootprint, ProductGroup, ProductGroupData,
    RepairabilityScore, UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport,
};
use crate::domain::status::PassportStatus;
use crate::schemas::lens::LensRegistry;
use chrono::Utc;
use uuid::Uuid;

fn make_passport() -> Passport {
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
fn unsold_goods_without_commodity_code_fails_validation() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = None;
    p.commodity_code = None;
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("commodity_code is required"), "got: {err}");
}

#[test]
fn unsold_goods_with_out_of_scope_commodity_code_fails_validation() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = None;
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("851712").expect("valid code"));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("not within ESPR Annex VII scope"),
        "got: {err}"
    );
}

#[test]
fn unsold_goods_with_annex_vii_commodity_code_passes_the_scope_check() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = None;
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("620342").expect("valid code"));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn missing_commodity_code_is_fine_outside_unsold_goods() {
    let mut p = make_passport(); // product_group = Electronics
    p.commodity_code = None;
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

fn unsold_goods_report(product_category: &str) -> UnsoldGoodsReport {
    UnsoldGoodsReport {
        reporting_period: "2026-Q3".to_owned(),
        volume_kg: 120.0,
        product_category: product_category.to_owned(),
        reason: UnsoldGoodsReason::EndOfSeason,
        destination: UnsoldGoodsDestination::Donation,
        destruction_justification: None,
        country_of_disposal: "DE".to_owned(),
        operator_name: Some("Caritas Berlin".to_owned()),
    }
}

#[test]
fn unsold_goods_category_matching_the_commodity_code_heading_passes() {
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(unsold_goods_report(
        "apparel",
    )));
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("620342").expect("valid code"));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn unsold_goods_accessories_matches_the_apparel_heading_too() {
    // Annex VII has one heading for apparel & clothing accessories, not two —
    // "accessories" must be accepted alongside "apparel" for the same code.
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(unsold_goods_report(
        "accessories",
    )));
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("650400").expect("valid code"));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn unsold_goods_category_contradicting_the_commodity_code_heading_fails() {
    // Footwear commodity code, apparel category word — same passport, two
    // fields describing the product, disagreeing with each other.
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(unsold_goods_report(
        "apparel",
    )));
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("64011000").expect("valid code"));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("does not match the Annex VII heading"),
        "got: {err}"
    );
}

#[test]
fn unsold_goods_home_textile_category_always_contradicts_annex_vii_scope() {
    // "home-textile" has no Annex VII heading at all, so it can never be
    // consistent with a commodity_code that (per the scope check above) must
    // already be apparel- or footwear-headed.
    let mut p = make_passport();
    p.product_group = ProductGroup::UnsoldGoods;
    p.product_group_data = Some(ProductGroupData::UnsoldGoods(unsold_goods_report(
        "home-textile",
    )));
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("620342").expect("valid code"));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("does not match the Annex VII heading"),
        "got: {err}"
    );
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

// ── validate() tests ──────────────────────────────────────────────

#[test]
fn validate_valid_passport_ok() {
    let p = make_passport();
    assert!(p.validate().is_ok());
}

#[test]
fn validate_empty_product_name() {
    let mut p = make_passport();
    p.product_name = "".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("product_name"), "got: {err}");
}

/// The placing-on-market date has two homes and they may not contradict.
///
/// It selects which law binds the product, so two answers is two different sets
/// of obligations and nothing downstream can tell which was meant. The envelope
/// field is the one a determination reads; battery's own predates it and is in
/// released schemas.
#[test]
fn validate_rejects_two_disagreeing_placing_on_market_dates() {
    let day = |d: u32| chrono::NaiveDate::from_ymd_opt(2031, 8, d).expect("valid date");
    let mut p = make_passport();
    p.product_group = ProductGroup::Battery;
    p.product_group_data = Some(ProductGroupData::Battery(Box::new(BatteryData {
        placed_on_market_date: Some(day(18)),
        ..crate::test_support::sample_battery_data()
    })));

    p.placed_on_market_date = Some(day(17));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("placed_on_market_date") && err.contains("two values"),
        "got: {err}"
    );

    // Agreeing is fine, and so is either one being absent — the envelope field
    // is optional and a passport that declares neither has simply not said.
    p.placed_on_market_date = Some(day(18));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
    p.placed_on_market_date = None;
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn validate_empty_manufacturer_name() {
    let mut p = make_passport();
    p.manufacturer.name = "   ".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("manufacturer.name"), "got: {err}");
}

#[test]
fn validate_empty_manufacturer_address() {
    let mut p = make_passport();
    p.manufacturer.address = "".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("manufacturer.address"), "got: {err}");
}

#[test]
fn validate_invalid_semver() {
    let mut p = make_passport();
    p.schema_version = "v1".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("schema_version"), "got: {err}");
}

#[test]
fn validate_rejects_vacuous_semver() {
    // ".5.0" (empty major) and "1.0.abc" (non-numeric patch) previously slipped
    // past the hand-rolled digit check and then silently skipped schema
    // validation downstream.
    for bad in [".5.0", "1.0.abc", "1.0", "1"] {
        let mut p = make_passport();
        p.schema_version = bad.to_owned();
        let err = p.validate().unwrap_err().to_string();
        assert!(
            err.contains("schema_version"),
            "schema_version '{bad}' should be rejected, got: {err}"
        );
    }

    // Pre-release / build metadata are still valid semver and must be accepted.
    let mut p = make_passport();
    p.schema_version = "1.0.0-alpha".to_owned();
    assert!(
        p.validate().is_ok(),
        "pre-release semver should be accepted"
    );
}

#[test]
fn validate_negative_co2e() {
    let mut p = make_passport();
    p.co2e_per_unit = Some(CarbonFootprint::from_kg(-1.0));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("co2e_per_unit"), "got: {err}");
}

#[test]
fn validate_repairability_out_of_range() {
    let mut p = make_passport();
    p.repairability_score = Some(RepairabilityScore::from_scalar(11.0));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("repairability_score"), "got: {err}");
}

#[test]
fn validate_multiple_errors_joined() {
    let mut p = make_passport();
    p.product_name = "".to_owned();
    p.manufacturer.name = "".to_owned();
    p.co2e_per_unit = Some(CarbonFootprint::from_kg(-5.0));
    let err = p.validate().unwrap_err().to_string();
    // All three issues should appear, separated by semicolons
    assert!(err.contains("product_name"), "got: {err}");
    assert!(err.contains("manufacturer.name"), "got: {err}");
    assert!(err.contains("co2e_per_unit"), "got: {err}");
}

#[test]
fn validate_none_optionals_ok() {
    let mut p = make_passport();
    p.co2e_per_unit = None;
    p.repairability_score = None;
    assert!(p.validate().is_ok());
}

#[test]
fn v02_fields_round_trip() {
    let mut p = make_passport();
    let predecessor_id = PassportId(Uuid::now_v7());
    p.version = 2;
    p.supersedes_id = Some(predecessor_id);
    p.retention_until = Some(Utc::now() + chrono::Duration::days(3650));
    p.product_id = Some(Uuid::now_v7());
    p.operator_identifier = Some("DE12345678".to_owned());
    p.facility = Some(crate::domain::passport::FacilitySnapshot {
        scheme: "national".to_owned(),
        value: "FAC-001".to_owned(),
        name: "Plant One".to_owned(),
        country: "DE".to_owned(),
        address: None,
    });

    let json = serde_json::to_string(&p).unwrap();
    let back: Passport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, 2);
    assert_eq!(back.supersedes_id, Some(predecessor_id));
    assert!(back.retention_until.is_some());
    assert_eq!(back.operator_identifier.as_deref(), Some("DE12345678"));
    assert_eq!(
        back.facility.as_ref().map(|f| f.value.as_str()),
        Some("FAC-001")
    );
    assert_eq!(
        back.facility.as_ref().map(|f| f.name.as_str()),
        Some("Plant One")
    );
}

#[test]
fn published_to_superseded_is_valid_transition() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Superseded).unwrap();
    assert_eq!(p.status, PassportStatus::Superseded);
}

#[test]
fn superseded_is_terminal() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Superseded).unwrap();
    assert!(p.transition_to(PassportStatus::Published).is_err());
    assert!(p.transition_to(PassportStatus::Archived).is_err());
    assert!(p.transition_to(PassportStatus::Draft).is_err());
}

#[test]
fn default_version_is_one_and_skipped_when_none_optional_fields_absent() {
    let p = make_passport();
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["version"], 1);
    assert!(json.get("supersedes_id").is_none() || json["supersedes_id"].is_null());
    assert!(json.get("retentionUntil").is_none() || json["retentionUntil"].is_null());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_wires_product_group_data_validation() {
    use crate::domain::product_group::{FibreEntry, TextileData};
    let mut p = make_passport();
    p.product_group = ProductGroup::Textile;
    p.product_group_data = Some(ProductGroupData::Textile(Box::new(TextileData {
        // fibre sum = 50% — cross-field rule must catch this
        fibre_composition: vec![FibreEntry {
            fibre: "cotton".into(),
            pct: 50.0,
            country_of_origin: None,
        }],
        country_of_origin: "DE".into(),
        chemical_compliance_standard: "REACH".into(),
        ..crate::test_support::sample_textile_data()
    })));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("fibreComposition") || err.contains("fibre"),
        "expected fibre error from product_group_data validation, got: {err}"
    );
}

#[test]
fn product_group_data_preserved_round_trip() {
    let mut passport = make_passport();
    passport.product_group = ProductGroup::Battery; // keep product_group consistent with the data
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(BatteryData {
        state_of_health_pct: Some(95.3),
        rated_capacity_kwh: Some(32.0),
        ..crate::test_support::sample_battery_data()
    })));
    let json = serde_json::to_string(&passport).unwrap();
    let back: Passport = serde_json::from_str(&json).unwrap();
    if let Some(ProductGroupData::Battery(ref b)) = back.product_group_data {
        assert_eq!(b.battery_chemistry, BatteryChemistry::Lfp);
        assert_eq!(b.state_of_health_pct, Some(95.3));
    } else {
        panic!("expected Battery product_group data");
    }
}

// ── redact() tests ────────────────────────────────────────────────────

fn battery_passport_with_due_diligence() -> Passport {
    let mut p = make_passport();
    p.product_group = ProductGroup::Battery;
    p.batch_id = Some("BATCH-42".into());
    p.jws_signature = Some("eyJhbGci.test.signature".into());
    p.product_group_data = Some(ProductGroupData::Battery(Box::new(BatteryData {
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        ..crate::test_support::sample_battery_data()
    })));
    p
}

#[test]
fn redact_public_strips_batch_id_jws_and_retention() {
    let catalog = crate::catalog::ProductGroupCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(Audience::Public, &catalog).into_value();
    assert!(
        view.get("batchId").is_none(),
        "batchId must be stripped at Public"
    );
    assert!(
        view.get("jwsSignature").is_none(),
        "jwsSignature must be stripped at Public"
    );
    assert!(
        view.get("retentionLocked").is_none(),
        "retentionLocked must be stripped at Public"
    );
    assert!(
        view.get("productName").is_some(),
        "productName must survive"
    );
}

#[test]
fn redact_public_strips_gated_product_group_fields() {
    let catalog = crate::catalog::ProductGroupCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(Audience::Public, &catalog).into_value();
    let sd = &view["productGroupData"];
    assert!(
        sd.get("dueDiligenceUrl").is_some(),
        "dueDiligenceUrl is Annex XIII point 1(d) — publicly accessible"
    );
    assert!(
        sd.get("disassemblyInstructionsUrl").is_none(),
        "disassemblyInstructionsUrl is Annex XIII point 2(c) — withheld from the public"
    );
    assert!(
        sd.get("batteryChemistry").is_some(),
        "batteryChemistry is Public — must survive"
    );
    assert!(
        sd.get("co2ePerUnitKg").is_some(),
        "co2ePerUnitKg is Public — must survive"
    );
}

#[test]
fn redact_professional_exposes_gated_product_group_fields() {
    let catalog = crate::catalog::ProductGroupCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p
        .redact(Audience::LegitimateInterest, &catalog)
        .into_value();
    let sd = &view["productGroupData"];
    assert!(
        sd.get("dueDiligenceUrl").is_some(),
        "Professional must see dueDiligenceUrl"
    );
    assert!(sd.get("disassemblyInstructionsUrl").is_some());
    // Still no JWS / retentionLocked at Professional
    assert!(view.get("jwsSignature").is_none());
    assert!(view.get("retentionLocked").is_none());
    // But batchId is visible
    assert!(view.get("batchId").is_some());
}

#[test]
fn redact_confidential_exposes_everything() {
    let catalog = crate::catalog::ProductGroupCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(Audience::Authority, &catalog).into_value();
    assert!(view.get("batchId").is_some());
    assert!(view.get("jwsSignature").is_some());
    assert!(view.get("retentionLocked").is_some());
    let sd = &view["productGroupData"];
    assert!(sd.get("dueDiligenceUrl").is_some());
}

#[test]
fn redact_no_product_group_data_leaves_passport_fields() {
    let catalog = crate::catalog::ProductGroupCatalog::new();
    let p = make_passport(); // no product_group_data, no batchId
    let view = p.redact(Audience::Public, &catalog).into_value();
    assert!(view.get("productName").is_some());
    assert!(view.get("productGroupData").is_none());
}

#[test]
fn redact_unknown_product_group_withholds_product_group_data_below_confidential() {
    let catalog = crate::catalog::ProductGroupCatalog::new();
    let mut p = make_passport();
    // `Other` maps to catalog key "other", which is absent from the embedded
    // catalog — so there are no per-field disclosure classes to redact against.
    p.product_group = ProductGroup::Other("other".into());
    p.product_group_data = Some(
        ProductGroupData::other(serde_json::json!({ "secretField": "leak-me" }))
            .expect("an untagged payload has no typed variant"),
    );

    // Public: no descriptor → withhold product group data entirely (fail closed).
    let public = p.redact(Audience::Public, &catalog).into_value();
    assert!(
        public["productGroupData"].is_null(),
        "unknown-product_group data must be withheld below Confidential, got: {}",
        public["productGroupData"]
    );
    assert!(
        !public.to_string().contains("leak-me"),
        "confidential product_group field leaked to a Public viewer"
    );

    // Confidential: sees every field anyway → full data.
    let conf = p.redact(Audience::Authority, &catalog).into_value();
    assert_eq!(conf["productGroupData"]["secretField"], "leak-me");
}

#[test]
fn public_view_omits_every_non_public_passport_field() {
    // Regression: `redact` carried its own field list and omitted `lintResult`,
    // which the crypto layer's policy classified as Restricted. A public view
    // built through the domain path disclosed it. Both now read one table, and
    // this asserts the property rather than the three fields that were listed.
    use crate::domain::identity::PASSPORT_FIELD_DISCLOSURE;

    let mut passport = make_passport();
    // Populate every non-public field so absence in the view proves redaction,
    // not that the field was simply unset.
    passport.batch_id = Some("BATCH-42".into());
    passport.jws_signature = Some("eyJhbGci.test.signature".into());
    passport.retention_locked = true;
    passport.lint_result = Some(crate::domain::lint::LintResult {
        pack_version: "test".into(),
        findings: Vec::new(),
        assessed_at: chrono::Utc::now(),
    });

    let catalog = crate::catalog::ProductGroupCatalog::new();
    let value = passport.redact(Audience::Public, &catalog).into_value();
    let obj = value.as_object().expect("view is an object");

    for (field, class) in PASSPORT_FIELD_DISCLOSURE {
        if !Audience::Public.may_see(*class) {
            assert!(
                !obj.contains_key(*field),
                "public view must not contain '{field}'"
            );
        }
    }

    // Guard against a vacuous pass: each field must actually be present for an
    // audience entitled to it, otherwise absence above proves nothing.
    let authority = passport.redact(Audience::Authority, &catalog).into_value();
    let authority = authority.as_object().expect("view is an object");
    for (field, class) in PASSPORT_FIELD_DISCLOSURE {
        if Audience::Authority.may_see(*class) {
            assert!(
                authority.contains_key(*field),
                "'{field}' should be visible to an authority; absence above would be vacuous"
            );
        }
    }
}

// ── Passport::from_stored ──────────────────────────────────────────────

fn textile_passport() -> Passport {
    Passport {
        product_group: ProductGroup::Textile,
        product_group_data: Some(ProductGroupData::Textile(Box::new(
            crate::test_support::sample_textile_data(),
        ))),
        schema_version: "1.2.0".into(),
        ..make_passport()
    }
}

#[test]
fn from_stored_reads_current_shape_directly() {
    let passport = textile_passport();
    let doc = serde_json::to_value(&passport).expect("serialise");
    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();

    let back = Passport::from_stored(doc, &lenses, &catalog).expect("current shape reads as-is");
    assert_eq!(back.id, passport.id);
    assert_eq!(back.product_group_data, passport.product_group_data);
}

#[test]
fn from_stored_upcasts_a_legacy_country_field() {
    // A real textile 1.1.0 document: same schema, old country-of-origin key.
    // The 1.1.0 -> 1.2.0 lens exists for exactly this rename.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["schemaVersion"] = "1.1.0".into();
    let country = doc["productGroupData"]["countryOfOrigin"].take();
    doc["productGroupData"]["countryOfManufacturing"] = country;

    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();
    let back = Passport::from_stored(doc, &lenses, &catalog)
        .expect("the registered lens bridges 1.1.0 -> 1.2.0");

    let Some(ProductGroupData::Textile(textile)) = back.product_group_data else {
        panic!("expected textile product_group data");
    };
    assert_eq!(textile.country_of_origin, "PT");
}

#[test]
fn from_stored_refuses_a_gap_no_lens_bridges() {
    // A document recorded at a version no lens leaves must fail loudly and
    // typed — not panic, and not silently pass through as if it were current.
    //
    // The version is one this build has never served. Pointing this at a real
    // unbridged version instead couples a test about refusal semantics to the
    // registry's lens coverage, so legitimately bridging that version fails a
    // test that was never about it — which is what happened when textile
    // 1.0.0 -> 1.1.0 was added and this test still named 1.0.0.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["schemaVersion"] = "0.9.0".into();
    let country = doc["productGroupData"]["countryOfOrigin"].take();
    doc["productGroupData"]["countryOfManufacturing"] = country;

    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();
    let err = Passport::from_stored(doc, &lenses, &catalog)
        .expect_err("no lens chain reaches the current version from 0.9.0");
    assert!(
        matches!(err, DppError::SchemaIncompatible(_)),
        "expected a typed SchemaIncompatible refusal, got: {err}"
    );
}

#[test]
fn from_stored_surfaces_a_same_version_mismatch_as_serialisation() {
    // schemaVersion already matches current, so there is no version gap to
    // blame — a genuine shape mismatch, not a compatibility question.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["productGroupData"]
        .as_object_mut()
        .unwrap()
        .remove("gtin");

    let lenses = LensRegistry::new();
    let catalog = ProductGroupCatalog::new();
    let err = Passport::from_stored(doc, &lenses, &catalog)
        .expect_err("gtin is required and there is no version gap to bridge");
    assert!(
        matches!(err, DppError::Serialisation(_)),
        "expected a typed Serialisation error, got: {err}"
    );
}

// ── The publish gate on mandatory battery content ────────────────────────────
//
// Blocking a publish is a serious act, so these cover the boundaries rather
// than the happy path: which category owes what, when the gate does *not* fire,
// and the one hole that is deliberate.

/// Every field the EV category makes mandatory, so a test can remove exactly
/// one and attribute the refusal to it.
fn publishable_battery(battery_type: crate::domain::product_group::BatteryType) -> Passport {
    use crate::domain::product_group::{
        BatteryStatus, DynamicPerformance, HazardousSubstance, MaterialComposition, StateOfHealth,
        TemperatureRange,
    };
    let range = TemperatureRange {
        min_c: -20.0,
        max_c: 60.0,
    };
    let mat = || {
        Some(vec![MaterialComposition {
            name: "LiFePO4".into(),
            weight_pct: 100.0,
            cas_number: None,
        }])
    };
    let data = BatteryData {
        battery_type,
        battery_weight_kg: Some(400.0),
        hazardous_substances: Some(vec![HazardousSubstance {
            name: "Nickel sulfate".into(),
            cas_number: None,
            concentration_pct: None,
        }]),
        usable_extinguishing_agent: Some("Class D dry powder".into()),
        critical_raw_materials: Some(vec![]),
        recycled_content_cobalt_pct: Some(4.0),
        recycled_content_lithium_pct: Some(4.0),
        recycled_content_nickel_pct: Some(4.0),
        recycled_content_lead_pct: Some(0.0),
        renewable_content_pct: Some(10.0),
        minimal_voltage_v: Some(2.5),
        maximum_voltage_v: Some(4.2),
        original_power_capability_w: Some(150_000.0),
        power_limit_min_w: Some(1_000.0),
        power_limit_max_w: Some(180_000.0),
        expected_lifetime_cycles: Some(3000),
        expected_lifetime_reference_test: Some("IEC 62660-1:2018".into()),
        capacity_threshold_for_exhaustion_pct: Some(80.0),
        not_in_use_temperature_range: Some(range),
        not_in_use_temperature_reference_test: Some("IEC 62660-1:2018".into()),
        initial_round_trip_efficiency_pct: Some(96.0),
        round_trip_efficiency_at_half_cycle_life_pct: Some(92.0),
        internal_cell_resistance_mohm: Some(1.2),
        internal_pack_resistance_mohm: Some(30.0),
        cycle_life_test_c_rate: Some(1.0),
        marking_information: Some("Separate collection symbol".into()),
        eu_declaration_of_conformity: Some("DoC-2027-0001".into()),
        waste_battery_information: Some("https://example.invalid/waste".into()),
        cathode_material: mat(),
        anode_material: mat(),
        electrolyte_material: mat(),
        component_part_numbers: Some(vec!["PN-1".into()]),
        spare_parts_contacts: Some("spares@example.invalid".into()),
        disassembly_instructions_url: Some("https://example.invalid/disassembly".into()),
        safety_measures: Some("Do not puncture".into()),
        test_report_results: Some("Report 42: pass".into()),
        dynamic_performance: Some(Box::new(DynamicPerformance::default())),
        state_of_health: Some(Box::new(StateOfHealth::ElectricVehicle { soce_pct: 99.0 })),
        battery_status: Some(BatteryStatus::Original),
        // Guidance data points 1, 7, 8 and 9 — mandatory for every covered
        // category, and unrepresentable until v2.6.0 declared them.
        battery_passport_number: Some("URN:UUID:6F1C9D2E-0000-4000-8000-000000000000".into()),
        battery_model_id: Some("LFP-64-A".into()),
        manufacturing_place: Some("PL:Wrocław".into()),
        manufacturing_date: Some(
            chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 3, 1, 0, 0, 0).unwrap(),
        ),
        ..crate::test_support::sample_battery_data()
    };
    Passport {
        product_group: ProductGroup::Battery,
        product_group_data: Some(ProductGroupData::Battery(Box::new(data))),
        ..crate::test_support::sample_passport()
    }
}

fn battery_field(p: &mut Passport, mutate: impl FnOnce(&mut BatteryData)) {
    if let Some(ProductGroupData::Battery(b)) = p.product_group_data.as_mut() {
        mutate(b);
    }
}

#[test]
fn a_complete_ev_battery_publishes() {
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    p.transition_to(PassportStatus::Published)
        .expect("a passport carrying every mandatory field must publish");
    assert!(p.retention_locked);
    assert!(p.published_at.is_some());
}

#[test]
fn a_missing_mandatory_field_blocks_names_it_and_leaves_no_lock() {
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);

    let err = p
        .transition_to(PassportStatus::Published)
        .expect_err("Annex VI Part A point 9 is mandatory for every covered category");
    let msg = err.to_string();
    assert!(
        msg.contains("usableExtinguishingAgent"),
        "the refusal must name the field: {msg}"
    );
    // A refused publish must leave nothing behind. Retention lock is permanent,
    // so setting it on a failed attempt would make the passport unrepairable.
    assert!(!p.retention_locked, "a refused publish must not lock");
    assert!(p.published_at.is_none());
    assert_eq!(p.status, PassportStatus::Draft);
}

#[test]
fn the_preview_gives_the_same_answer_as_the_attempt_and_changes_nothing() {
    // The gate is reachable without attempting the transition, and asking is
    // not declining: the preview returns the refusal verbatim, and the passport
    // is untouched either way.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);

    let previewed = p
        .check_mandatory_content()
        .expect_err("the preview must refuse what the transition refuses");

    // Asking left no trace.
    assert_eq!(p.status, PassportStatus::Draft);
    assert!(!p.retention_locked);
    assert!(p.published_at.is_none());

    let attempted = p
        .transition_to(PassportStatus::Published)
        .expect_err("the transition must still refuse");
    assert_eq!(
        previewed.to_string(),
        attempted.to_string(),
        "a preview that does not render identically to the refusal is a second \
         opinion, and the two would drift"
    );
}

#[test]
fn the_preview_passes_for_a_passport_that_publishes() {
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    p.check_mandatory_content()
        .expect("a complete passport must preview clean");
    p.transition_to(PassportStatus::Published)
        .expect("and must then actually publish");
}

/// The four identity data points the guidance marks mandatory for every
/// covered category each block a publish on their own.
///
/// These were absent from the requirements table *and* from every battery
/// schema property, so a passport could be published carrying none of
/// them: no unique identifier, no model identification, and no record of where
/// or when the battery was made. The schema could not even store them —
/// `additionalProperties: false` rejected all four — so this test is the guard
/// on both halves of that defect at once. It fails if either the requirements
/// row or the schema property is removed.
#[test]
fn each_identity_data_point_blocks_publish_on_its_own() {
    for (name, clear) in [
        (
            "batteryPassportNumber",
            (|b: &mut BatteryData| b.battery_passport_number = None) as fn(&mut BatteryData),
        ),
        ("batteryModelId", |b: &mut BatteryData| {
            b.battery_model_id = None;
        }),
        ("manufacturingPlace", |b: &mut BatteryData| {
            b.manufacturing_place = None;
        }),
        ("manufacturingDate", |b: &mut BatteryData| {
            b.manufacturing_date = None;
        }),
    ] {
        let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
        battery_field(&mut p, clear);

        let err = match p.transition_to(PassportStatus::Published) {
            Err(e) => e,
            Ok(()) => panic!(
                "{name} is mandatory for every covered category, but the publish was allowed"
            ),
        };
        let msg = format!("{err:?}");
        assert!(msg.contains(name), "the refusal must name {name}: {msg}");
        assert!(!p.retention_locked, "a refused publish must not lock");
        assert_eq!(p.status, PassportStatus::Draft);
    }
}

#[test]
fn every_missing_field_is_reported_at_once() {
    // One-at-a-time reporting turns a single fix into N publish attempts.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| {
        b.usable_extinguishing_agent = None;
        b.marking_information = None;
    });
    let msg = p
        .transition_to(PassportStatus::Published)
        .unwrap_err()
        .to_string();
    assert!(msg.contains("usableExtinguishingAgent"), "{msg}");
    assert!(msg.contains("markingInformation"), "{msg}");
}

#[test]
fn the_ev_only_field_is_demanded_of_ev_and_not_of_lmt() {
    // Annex XIII point 1(k): mandatory for EV, "not to be filled/displayed" for
    // LMT and industrial. The sharpest per-category split in the guidance and
    // the one most easily flattened by a careless edit.
    let mut ev = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut ev, |b| b.capacity_threshold_for_exhaustion_pct = None);
    assert!(
        ev.transition_to(PassportStatus::Published).is_err(),
        "1(k) is mandatory for EV"
    );

    let mut lmt = publishable_battery(crate::domain::product_group::BatteryType::Lmt);
    battery_field(&mut lmt, |b| b.capacity_threshold_for_exhaustion_pct = None);
    lmt.transition_to(PassportStatus::Published)
        .expect("1(k) is not applicable to LMT, so its absence cannot block");
}

#[test]
fn industrial_may_publish_without_a_cycle_lifetime() {
    // Point 1(j) reaches industrial batteries only "where lifetime can be
    // expressed in cycles". That carve-out is why the field became optional;
    // the gate must not quietly reintroduce the requirement.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Industrial);
    battery_field(&mut p, |b| {
        b.expected_lifetime_cycles = None;
        b.expected_lifetime_reference_test = None;
        b.cycle_life_test_c_rate = None;
        b.initial_round_trip_efficiency_pct = None;
        b.round_trip_efficiency_at_half_cycle_life_pct = None;
    });
    p.transition_to(PassportStatus::Published)
        .expect("every field removed here is conditional for industrial batteries");
}

#[test]
fn portable_and_sli_are_ungated_and_that_is_deliberate() {
    // The guidance covers EV, LMT and industrial only. Blocking a portable
    // battery would invent a requirement the source declines to state — the
    // defect class this project keeps catching in other people's work. A real
    // hole, held open on purpose until a source covering them exists.
    for t in [
        crate::domain::product_group::BatteryType::Portable,
        crate::domain::product_group::BatteryType::Sli,
    ] {
        let mut p = publishable_battery(t);
        battery_field(&mut p, |b| {
            b.usable_extinguishing_agent = None;
            b.marking_information = None;
            b.eu_declaration_of_conformity = None;
        });
        p.transition_to(PassportStatus::Published)
            .expect("no source covers this category, so nothing is gated");
    }
}

#[test]
fn a_republish_is_not_re_gated() {
    // `transition_to` also runs on Suspended → Published. Gating a republish
    // would let a later change to the requirements table strand a passport
    // published lawfully under the earlier one — and retention lock means the
    // operator could not repair it. Content is judged once, at first publish.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Suspended).unwrap();

    // Stand in for the table tightening under an already-published record.
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);

    p.transition_to(PassportStatus::Published)
        .expect("a republish must not be blocked by a rule that arrived after issuance");
}

#[test]
fn a_battery_passport_without_product_group_data_cannot_publish() {
    let mut p = Passport {
        product_group: ProductGroup::Battery,
        product_group_data: None,
        ..crate::test_support::sample_passport()
    };
    let err = p.transition_to(PassportStatus::Published).unwrap_err();
    assert!(err.to_string().contains("productGroupData"), "{err}");
}

#[test]
fn a_non_battery_product_group_is_untouched_by_the_gate() {
    let mut p = crate::test_support::sample_passport();
    p.product_group = ProductGroup::Textile;
    p.product_group_data = None;
    p.transition_to(PassportStatus::Published)
        .expect("no requirements table exists for textile, so the gate is inert");
}

#[test]
fn a_non_publish_transition_is_not_gated() {
    // Only publish is judged. An incomplete draft must stay movable, or an
    // operator cannot abandon one they decided not to finish.
    let mut p = publishable_battery(crate::domain::product_group::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);
    p.transition_to(PassportStatus::Archived)
        .expect("archiving an incomplete draft is not a compliance claim");
}
