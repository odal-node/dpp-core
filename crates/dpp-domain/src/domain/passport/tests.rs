//! Serde round-trip, state-machine, validation, and redaction tests for `Passport`.

use super::*;
use crate::catalog::SectorCatalog;
use crate::domain::error::DppError;
use crate::domain::identity::Audience;
use crate::domain::sector::{
    BatteryChemistry, BatteryData, CarbonFootprint, RepairabilityScore, Sector, SectorData,
    UnsoldGoodsDestination, UnsoldGoodsReason, UnsoldGoodsReport,
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
        sector: Sector::Electronics,
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
fn passport_carries_typed_sector() {
    let json = serde_json::to_value(make_passport()).expect("serialise");
    assert_eq!(json["sector"], "electronics"); // Sector → camelCase
    let back: Passport = serde_json::from_value(json).expect("deserialise");
    assert_eq!(back.sector, Sector::Electronics);
}

#[test]
fn sector_data_mismatch_fails_validation() {
    let mut p = make_passport(); // sector = Electronics
    p.sector_data = Some(SectorData::Battery(Box::new(
        crate::test_support::sample_battery_data(),
    )));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("sector must match"), "got: {err}");
}

#[test]
fn unsold_goods_without_commodity_code_fails_validation() {
    let mut p = make_passport();
    p.sector = Sector::UnsoldGoods;
    p.sector_data = None;
    p.commodity_code = None;
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("commodity_code is required"), "got: {err}");
}

#[test]
fn unsold_goods_with_out_of_scope_commodity_code_fails_validation() {
    let mut p = make_passport();
    p.sector = Sector::UnsoldGoods;
    p.sector_data = None;
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
    p.sector = Sector::UnsoldGoods;
    p.sector_data = None;
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("620342").expect("valid code"));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn missing_commodity_code_is_fine_outside_unsold_goods() {
    let mut p = make_passport(); // sector = Electronics
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
    p.sector = Sector::UnsoldGoods;
    p.sector_data = Some(SectorData::UnsoldGoods(unsold_goods_report("apparel")));
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("620342").expect("valid code"));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn unsold_goods_accessories_matches_the_apparel_heading_too() {
    // Annex VII has one heading for apparel & clothing accessories, not two —
    // "accessories" must be accepted alongside "apparel" for the same code.
    let mut p = make_passport();
    p.sector = Sector::UnsoldGoods;
    p.sector_data = Some(SectorData::UnsoldGoods(unsold_goods_report("accessories")));
    p.commodity_code =
        Some(crate::domain::commodity_code::CommodityCode::parse("650400").expect("valid code"));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn unsold_goods_category_contradicting_the_commodity_code_heading_fails() {
    // Footwear commodity code, apparel category word — same passport, two
    // fields describing the product, disagreeing with each other.
    let mut p = make_passport();
    p.sector = Sector::UnsoldGoods;
    p.sector_data = Some(SectorData::UnsoldGoods(unsold_goods_report("apparel")));
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
    p.sector = Sector::UnsoldGoods;
    p.sector_data = Some(SectorData::UnsoldGoods(unsold_goods_report("home-textile")));
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
fn validate_wires_sector_data_validation() {
    use crate::domain::sector::{FibreEntry, TextileData};
    let mut p = make_passport();
    p.sector = Sector::Textile;
    p.sector_data = Some(SectorData::Textile(Box::new(TextileData {
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
        "expected fibre error from sector_data validation, got: {err}"
    );
}

#[test]
fn sector_data_preserved_round_trip() {
    let mut passport = make_passport();
    passport.sector = Sector::Battery; // keep sector consistent with the data
    passport.sector_data = Some(SectorData::Battery(Box::new(BatteryData {
        state_of_health_pct: Some(95.3),
        rated_capacity_kwh: Some(32.0),
        ..crate::test_support::sample_battery_data()
    })));
    let json = serde_json::to_string(&passport).unwrap();
    let back: Passport = serde_json::from_str(&json).unwrap();
    if let Some(SectorData::Battery(ref b)) = back.sector_data {
        assert_eq!(b.battery_chemistry, BatteryChemistry::Lfp);
        assert_eq!(b.state_of_health_pct, Some(95.3));
    } else {
        panic!("expected Battery sector data");
    }
}

// ── redact() tests ────────────────────────────────────────────────────

fn battery_passport_with_due_diligence() -> Passport {
    let mut p = make_passport();
    p.sector = Sector::Battery;
    p.batch_id = Some("BATCH-42".into());
    p.jws_signature = Some("eyJhbGci.test.signature".into());
    p.sector_data = Some(SectorData::Battery(Box::new(BatteryData {
        due_diligence_url: Some("https://acme.example.com/due-diligence".into()),
        disassembly_instructions_url: Some("https://acme.example.com/disassembly".into()),
        ..crate::test_support::sample_battery_data()
    })));
    p
}

#[test]
fn redact_public_strips_batch_id_jws_and_retention() {
    let catalog = crate::catalog::SectorCatalog::new();
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
fn redact_public_strips_gated_sector_fields() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(Audience::Public, &catalog).into_value();
    let sd = &view["sectorData"];
    assert!(
        sd.get("dueDiligenceUrl").is_none(),
        "dueDiligenceUrl is Professional — must be hidden"
    );
    assert!(
        sd.get("disassemblyInstructionsUrl").is_none(),
        "disassemblyInstructionsUrl is Professional"
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
fn redact_professional_exposes_gated_sector_fields() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p
        .redact(Audience::LegitimateInterest, &catalog)
        .into_value();
    let sd = &view["sectorData"];
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
    let catalog = crate::catalog::SectorCatalog::new();
    let p = battery_passport_with_due_diligence();
    let view = p.redact(Audience::Authority, &catalog).into_value();
    assert!(view.get("batchId").is_some());
    assert!(view.get("jwsSignature").is_some());
    assert!(view.get("retentionLocked").is_some());
    let sd = &view["sectorData"];
    assert!(sd.get("dueDiligenceUrl").is_some());
}

#[test]
fn redact_no_sector_data_leaves_passport_fields() {
    let catalog = crate::catalog::SectorCatalog::new();
    let p = make_passport(); // no sector_data, no batchId
    let view = p.redact(Audience::Public, &catalog).into_value();
    assert!(view.get("productName").is_some());
    assert!(view.get("sectorData").is_none());
}

#[test]
fn redact_unknown_sector_withholds_sector_data_below_confidential() {
    let catalog = crate::catalog::SectorCatalog::new();
    let mut p = make_passport();
    // `Other` maps to catalog key "other", which is absent from the embedded
    // catalog — so there are no per-field disclosure classes to redact against.
    p.sector = Sector::Other("other".into());
    p.sector_data = Some(
        SectorData::other(serde_json::json!({ "secretField": "leak-me" }))
            .expect("an untagged payload has no typed variant"),
    );

    // Public: no descriptor → withhold sector data entirely (fail closed).
    let public = p.redact(Audience::Public, &catalog).into_value();
    assert!(
        public["sectorData"].is_null(),
        "unknown-sector data must be withheld below Confidential, got: {}",
        public["sectorData"]
    );
    assert!(
        !public.to_string().contains("leak-me"),
        "confidential sector field leaked to a Public viewer"
    );

    // Confidential: sees every field anyway → full data.
    let conf = p.redact(Audience::Authority, &catalog).into_value();
    assert_eq!(conf["sectorData"]["secretField"], "leak-me");
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

    let catalog = crate::catalog::SectorCatalog::new();
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
        sector: Sector::Textile,
        sector_data: Some(SectorData::Textile(Box::new(
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
    let catalog = SectorCatalog::new();

    let back = Passport::from_stored(doc, &lenses, &catalog).expect("current shape reads as-is");
    assert_eq!(back.id, passport.id);
    assert_eq!(back.sector_data, passport.sector_data);
}

#[test]
fn from_stored_upcasts_a_legacy_country_field() {
    // A real textile 1.1.0 document: same schema, old country-of-origin key.
    // The 1.1.0 -> 1.2.0 lens exists for exactly this rename.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["schemaVersion"] = "1.1.0".into();
    let country = doc["sectorData"]["countryOfOrigin"].take();
    doc["sectorData"]["countryOfManufacturing"] = country;

    let lenses = LensRegistry::new();
    let catalog = SectorCatalog::new();
    let back = Passport::from_stored(doc, &lenses, &catalog)
        .expect("the registered lens bridges 1.1.0 -> 1.2.0");

    let Some(SectorData::Textile(textile)) = back.sector_data else {
        panic!("expected textile sector data");
    };
    assert_eq!(textile.country_of_origin, "PT");
}

#[test]
fn from_stored_refuses_a_gap_no_lens_bridges() {
    // Textile's current version is 1.2.0, and today's registry only bridges
    // 1.1.0 -> 1.2.0 — nothing leaves 1.0.0. A document honestly recorded at
    // 1.0.0 (old country key, so the direct read genuinely fails) cannot be
    // upgraded, and must fail loudly and typed, not panic or silently pass
    // through as if it were current.
    let passport = textile_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc["schemaVersion"] = "1.0.0".into();
    let country = doc["sectorData"]["countryOfOrigin"].take();
    doc["sectorData"]["countryOfManufacturing"] = country;

    let lenses = LensRegistry::new();
    let catalog = SectorCatalog::new();
    let err = Passport::from_stored(doc, &lenses, &catalog)
        .expect_err("no lens chain reaches 1.2.0 from 1.0.0");
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
    doc["sectorData"].as_object_mut().unwrap().remove("gtin");

    let lenses = LensRegistry::new();
    let catalog = SectorCatalog::new();
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
fn publishable_battery(battery_type: crate::domain::sector::BatteryType) -> Passport {
    use crate::domain::sector::{
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
        ..crate::test_support::sample_battery_data()
    };
    Passport {
        sector: Sector::Battery,
        sector_data: Some(SectorData::Battery(Box::new(data))),
        ..crate::test_support::sample_passport()
    }
}

fn battery_field(p: &mut Passport, mutate: impl FnOnce(&mut BatteryData)) {
    if let Some(SectorData::Battery(b)) = p.sector_data.as_mut() {
        mutate(b);
    }
}

#[test]
fn a_complete_ev_battery_publishes() {
    let mut p = publishable_battery(crate::domain::sector::BatteryType::Ev);
    p.transition_to(PassportStatus::Published)
        .expect("a passport carrying every mandatory field must publish");
    assert!(p.retention_locked);
    assert!(p.published_at.is_some());
}

#[test]
fn a_missing_mandatory_field_blocks_names_it_and_leaves_no_lock() {
    let mut p = publishable_battery(crate::domain::sector::BatteryType::Ev);
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
fn every_missing_field_is_reported_at_once() {
    // One-at-a-time reporting turns a single fix into N publish attempts.
    let mut p = publishable_battery(crate::domain::sector::BatteryType::Ev);
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
    let mut ev = publishable_battery(crate::domain::sector::BatteryType::Ev);
    battery_field(&mut ev, |b| b.capacity_threshold_for_exhaustion_pct = None);
    assert!(
        ev.transition_to(PassportStatus::Published).is_err(),
        "1(k) is mandatory for EV"
    );

    let mut lmt = publishable_battery(crate::domain::sector::BatteryType::Lmt);
    battery_field(&mut lmt, |b| b.capacity_threshold_for_exhaustion_pct = None);
    lmt.transition_to(PassportStatus::Published)
        .expect("1(k) is not applicable to LMT, so its absence cannot block");
}

#[test]
fn industrial_may_publish_without_a_cycle_lifetime() {
    // Point 1(j) reaches industrial batteries only "where lifetime can be
    // expressed in cycles". That carve-out is why the field became optional;
    // the gate must not quietly reintroduce the requirement.
    let mut p = publishable_battery(crate::domain::sector::BatteryType::Industrial);
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
        crate::domain::sector::BatteryType::Portable,
        crate::domain::sector::BatteryType::Sli,
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
    let mut p = publishable_battery(crate::domain::sector::BatteryType::Ev);
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Suspended).unwrap();

    // Stand in for the table tightening under an already-published record.
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);

    p.transition_to(PassportStatus::Published)
        .expect("a republish must not be blocked by a rule that arrived after issuance");
}

#[test]
fn a_battery_passport_without_sector_data_cannot_publish() {
    let mut p = Passport {
        sector: Sector::Battery,
        sector_data: None,
        ..crate::test_support::sample_passport()
    };
    let err = p.transition_to(PassportStatus::Published).unwrap_err();
    assert!(err.to_string().contains("sectorData"), "{err}");
}

#[test]
fn a_non_battery_sector_is_untouched_by_the_gate() {
    let mut p = crate::test_support::sample_passport();
    p.sector = Sector::Textile;
    p.sector_data = None;
    p.transition_to(PassportStatus::Published)
        .expect("no requirements table exists for textile, so the gate is inert");
}

#[test]
fn a_non_publish_transition_is_not_gated() {
    // Only publish is judged. An incomplete draft must stay movable, or an
    // operator cannot abandon one they decided not to finish.
    let mut p = publishable_battery(crate::domain::sector::BatteryType::Ev);
    battery_field(&mut p, |b| b.usable_extinguishing_agent = None);
    p.transition_to(PassportStatus::Archived)
        .expect("archiving an incomplete draft is not a compliance claim");
}
