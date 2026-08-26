//! Redaction by audience: which fields each tier of viewer is served.

use super::*;
use crate::disclosure::Audience;
use crate::product_group::{BatteryData, ProductGroup, ProductGroupData};

use super::tests::make_passport;

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
    use crate::disclosure::PASSPORT_FIELD_DISCLOSURE;

    let mut passport = make_passport();
    // Populate every non-public field so absence in the view proves redaction,
    // not that the field was simply unset.
    passport.batch_id = Some("BATCH-42".into());
    passport.jws_signature = Some("eyJhbGci.test.signature".into());
    passport.retention_locked = true;
    passport.lint_result = Some(crate::lint::LintResult {
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
