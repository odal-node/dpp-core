//! Audience tiers: what each one sees, and what it never does.

use serde_json::json;

use crate::{Audience, Disclosure};

use super::filter::{filter_by_audience, filter_by_audience_in_scope};
use super::policy::{DocumentScope, ProductGroupAccessPolicy};

/// Filter a **bare product-group payload** — a document whose root is already
/// inside `productGroupData`, which is how the resolver filters it on its second
/// pass. Says so explicitly, because a payload filtered as an envelope would have
/// none of its product group's classes applied and would serve every restricted
/// field in it.
pub(super) fn filter_payload(
    data: &serde_json::Value,
    policy: &ProductGroupAccessPolicy,
    audience: Audience,
) -> super::filter::PolicyDecision {
    filter_by_audience_in_scope(data, policy, audience, DocumentScope::ProductGroupData)
}

/// The policy for a product group's current schema version — the path a served
/// passport actually takes.
pub(super) fn current_policy(product_group: &str) -> ProductGroupAccessPolicy {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let (version, _) = reg
        .latest(product_group)
        .expect("product_group has a schema");
    ProductGroupAccessPolicy::for_schema_version(product_group, &version.to_string())
        .expect("current version yields a policy")
}

pub(super) fn textile_policy() -> ProductGroupAccessPolicy {
    current_policy("textile")
}

pub(super) fn battery_policy() -> ProductGroupAccessPolicy {
    current_policy("battery")
}

pub(super) fn sample_textile_data() -> serde_json::Value {
    json!({
        "fibreComposition": [
            { "fibre": "cotton", "pct": 70.0 },
            { "fibre": "polyester", "pct": 30.0 }
        ],
        "countryOfManufacturing": "BD",
        "careInstructions": "Machine wash 40°C",
        "carbonFootprintKgCo2e": 8.5,
        "durabilityScore": 7.5,
        "svhcSubstances": [
            { "casNumber": "80-05-7", "substanceName": "Bisphenol A", "concentrationPct": 0.15 }
        ],
        "disassemblyInstructions": "Remove buttons, separate layers by colour",
        "sparePartsAvailable": true,
        "jwsSignature": "eyJhbGciOiJFZERTQSJ9...",
        "complianceReport": { "status": "compliant", "auditor": "TUV" }
    })
}

#[test]
fn public_tier_redacts_professional_and_confidential() {
    let policy = textile_policy();
    let data = sample_textile_data();
    let decision = filter_payload(&data, &policy, Audience::Public);

    assert!(decision.filtered_data["fibreComposition"].is_array());
    assert!(decision.filtered_data["countryOfManufacturing"].is_string());
    assert!(decision.filtered_data["carbonFootprintKgCo2e"].is_number());
    assert!(decision.filtered_data["durabilityScore"].is_number());

    assert!(decision.filtered_data.get("svhcSubstances").is_none());
    assert!(
        decision
            .filtered_data
            .get("disassemblyInstructions")
            .is_none()
    );
    assert!(decision.filtered_data.get("sparePartsAvailable").is_none());

    assert!(decision.filtered_data.get("jwsSignature").is_none());
    assert!(decision.filtered_data.get("complianceReport").is_none());

    assert!(
        decision
            .redacted_fields
            .contains(&"svhcSubstances".to_owned())
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"jwsSignature".to_owned())
    );
}

#[test]
fn professional_tier_sees_professional_fields() {
    let policy = textile_policy();
    let data = sample_textile_data();
    let decision = filter_payload(&data, &policy, Audience::LegitimateInterest);

    assert!(decision.filtered_data["svhcSubstances"].is_array());
    assert!(decision.filtered_data["disassemblyInstructions"].is_string());
    assert!(decision.filtered_data["sparePartsAvailable"].is_boolean());

    assert!(decision.filtered_data.get("jwsSignature").is_none());
    assert!(decision.filtered_data.get("complianceReport").is_none());

    assert!(
        !decision
            .redacted_fields
            .contains(&"svhcSubstances".to_owned())
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"jwsSignature".to_owned())
    );
}

#[test]
fn confidential_tier_sees_everything() {
    let policy = textile_policy();
    let data = sample_textile_data();
    let decision = filter_payload(&data, &policy, Audience::Authority);

    assert!(decision.redacted_fields.is_empty());
    assert!(decision.filtered_data["svhcSubstances"].is_array());
    assert!(decision.filtered_data["jwsSignature"].is_string());
    assert!(decision.filtered_data["complianceReport"].is_object());
}

#[test]
fn unknown_fields_default_to_public() {
    let policy = textile_policy();
    assert_eq!(
        policy.disclosure_for_field("fibreComposition"),
        Disclosure::Public
    );
    assert_eq!(
        policy.disclosure_for_field("unknownField"),
        Disclosure::Public
    );
}

/// The public view keeps Annex XIII point 1 content and drops point 2 content.
///
/// Both halves matter, and this test previously got one of them backwards. It
/// asserted that `dueDiligenceUrl` and `criticalRawMaterials` were redacted
/// from the public view, which does not match the annex:
///
/// - Point 1(b) lists "critical raw materials present in the battery" in the
///   same sentence as chemistry and hazardous substances, and Annex VI Part A
///   point 10 reaches the same result through point 1(a).
/// - Point 1(d) is "information on responsible sourcing as indicated in the
///   report on battery due diligence policy referred to in Article 52(3)".
///
/// Both sit in the publicly accessible tier. Over-redaction is not the safe
/// direction here: it makes the public passport omit content the regulation
/// requires it to carry, and a test asserting the omission makes that
/// permanent.
#[test]
fn battery_policy_public_keeps_point_1_and_drops_point_2() {
    let policy = battery_policy();
    let data = json!({
        "gtin": "09506000134352",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 400.0,
        "co2ePerUnitKg": 150.0,
        "dueDiligenceUrl": "https://example.com/due-diligence",
        "criticalRawMaterials": [{"casNumber": "7440-48-4", "name": "Cobalt"}],
        "disassemblyInstructionsUrl": "https://example.com/disassembly"
    });
    let decision = filter_payload(&data, &policy, Audience::Public);
    assert!(decision.filtered_data.get("gtin").is_some());
    // Point 1 — must survive.
    assert!(
        decision.filtered_data.get("dueDiligenceUrl").is_some(),
        "Annex XIII point 1(d) is publicly accessible"
    );
    assert!(
        decision.filtered_data.get("criticalRawMaterials").is_some(),
        "Annex XIII point 1(b) is publicly accessible"
    );
    // Point 2(c), dismantling information — must not.
    assert!(
        decision
            .filtered_data
            .get("disassemblyInstructionsUrl")
            .is_none(),
        "Annex XIII point 2(c) is withheld from the general public"
    );
}

#[test]
fn passport_policy_public_redacts_jws() {
    let policy = ProductGroupAccessPolicy::passport_default();
    let data = json!({
        "id": "abc-123",
        "productName": "Widget",
        "status": "active",
        "jwsSignature": "eyJhbGciOiJFZERTQSJ9...",
        "batchId": "BATCH-42"
    });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
    assert!(decision.filtered_data.get("productName").is_some());
    assert!(decision.filtered_data.get("jwsSignature").is_none());
    assert!(decision.filtered_data.get("batchId").is_none());
}

/// Locks the "nothing mutable-after-publish sits at `Public`" invariant for
/// `lintResult`. The public view is what `publicJwsSignature` is computed over,
/// and the lint result is deliberately re-computable after publish — serving it
/// at `Public` would make the live body stop verifying against its own frozen
/// signature for reasons that are not tampering. Other mutable-after-publish
/// keys (`status`, `publishedAt`, `updatedAt`, `qrCodeUrl`) stay `Public` by
/// design: they are lifecycle metadata, not compliance content.
#[test]
fn passport_default_keeps_lint_result_out_of_public_view() {
    let policy = ProductGroupAccessPolicy::passport_default();
    let data = json!({
        "id": "abc-123",
        "productName": "Widget",
        "status": "published",
        "lintResult": {
            "findings": [{ "code": "implausible_mass", "message": "net mass exceeds gross" }],
            "assessedAt": "2026-07-19T00:00:00Z"
        }
    });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
    assert!(
        decision.filtered_data.get("lintResult").is_none(),
        "lintResult is rewritten post-publish; it must not sit inside the signed public view"
    );
    assert_eq!(
        policy.disclosure_for_field("lintResult"),
        Disclosure::Restricted
    );
    // The domain field is `lint_result`; leaf matching is separator-insensitive,
    // so a snake_case payload must be gated identically.
    assert_eq!(
        policy.disclosure_for_field("lint_result"),
        Disclosure::Restricted
    );
    // Exempt-by-design lifecycle metadata is unaffected.
    assert_eq!(policy.disclosure_for_field("status"), Disclosure::Public);
    assert!(decision.filtered_data.get("status").is_some());
}

#[test]
fn non_object_input_returned_unchanged() {
    let policy = textile_policy();
    let data = json!("just a string");
    let decision = filter_by_audience(&data, &policy, Audience::Public);
    assert_eq!(decision.filtered_data, json!("just a string"));
    assert!(decision.redacted_fields.is_empty());
}

#[test]
fn policy_round_trip() {
    let policy = textile_policy();
    let json = serde_json::to_value(&policy).unwrap();
    let back: ProductGroupAccessPolicy = serde_json::from_value(json).unwrap();
    assert_eq!(back.name, "textile-1.2.0");
    assert_eq!(back.product_group, "textile");
    assert_eq!(
        back.disclosure_for_field("svhcSubstances"),
        Disclosure::Restricted
    );
}

#[test]
fn custom_policy_overrides_defaults() {
    let mut policy = textile_policy();
    policy
        .field_disclosure
        .insert("durabilityScore".into(), Disclosure::Restricted);

    let data = sample_textile_data();
    let decision = filter_payload(&data, &policy, Audience::Public);
    assert!(decision.filtered_data.get("durabilityScore").is_none());
    assert!(
        decision
            .redacted_fields
            .contains(&"durabilityScore".to_owned())
    );
}
