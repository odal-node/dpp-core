use std::collections::HashMap;

use dpp_domain::{AccessTier, SectorCatalog};
use serde_json::json;

use super::filter::filter_by_access_tier;
use super::policy::SectorAccessPolicy;

fn textile_policy() -> SectorAccessPolicy {
    SectorAccessPolicy::from_catalog(&SectorCatalog::new(), "textile").expect("textile in catalog")
}

fn battery_policy() -> SectorAccessPolicy {
    SectorAccessPolicy::from_catalog(&SectorCatalog::new(), "battery").expect("battery in catalog")
}

fn sample_textile_data() -> serde_json::Value {
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
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);

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
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Professional);

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
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Confidential);

    assert!(decision.redacted_fields.is_empty());
    assert!(decision.filtered_data["svhcSubstances"].is_array());
    assert!(decision.filtered_data["jwsSignature"].is_string());
    assert!(decision.filtered_data["complianceReport"].is_object());
}

#[test]
fn unknown_fields_default_to_public() {
    let policy = textile_policy();
    assert_eq!(
        policy.tier_for_field("fibreComposition"),
        AccessTier::Public
    );
    assert_eq!(policy.tier_for_field("unknownField"), AccessTier::Public);
}

#[test]
fn battery_policy_public_redacts_due_diligence() {
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
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert!(decision.filtered_data.get("gtin").is_some());
    assert!(decision.filtered_data.get("dueDiligenceUrl").is_none());
    assert!(decision.filtered_data.get("criticalRawMaterials").is_none());
    assert!(
        decision
            .filtered_data
            .get("disassemblyInstructionsUrl")
            .is_none()
    );
}

#[test]
fn passport_policy_public_redacts_jws() {
    let policy = SectorAccessPolicy::passport_default();
    let data = json!({
        "id": "abc-123",
        "productName": "Widget",
        "status": "active",
        "jwsSignature": "eyJhbGciOiJFZERTQSJ9...",
        "batchId": "BATCH-42"
    });
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert!(decision.filtered_data.get("productName").is_some());
    assert!(decision.filtered_data.get("jwsSignature").is_none());
    assert!(decision.filtered_data.get("batchId").is_none());
}

#[test]
fn non_object_input_returned_unchanged() {
    let policy = textile_policy();
    let data = json!("just a string");
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert_eq!(decision.filtered_data, json!("just a string"));
    assert!(decision.redacted_fields.is_empty());
}

#[test]
fn policy_round_trip() {
    let policy = textile_policy();
    let json = serde_json::to_value(&policy).unwrap();
    let back: SectorAccessPolicy = serde_json::from_value(json).unwrap();
    assert_eq!(back.name, "textile-1.1.0");
    assert_eq!(back.sector, "textile");
    assert_eq!(
        back.tier_for_field("svhcSubstances"),
        AccessTier::Professional
    );
}

#[test]
fn custom_policy_overrides_defaults() {
    let mut policy = textile_policy();
    policy
        .field_tiers
        .insert("durabilityScore".into(), AccessTier::Professional);

    let data = sample_textile_data();
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert!(decision.filtered_data.get("durabilityScore").is_none());
    assert!(
        decision
            .redacted_fields
            .contains(&"durabilityScore".to_owned())
    );
}

// ── crypto Gap 6: path-aware, fail-closed redaction ──────────────────────────

fn policy_with(name: &str, tier: AccessTier) -> SectorAccessPolicy {
    let mut field_tiers = HashMap::new();
    field_tiers.insert(name.to_owned(), tier);
    SectorAccessPolicy {
        name: "test".into(),
        sector: "test".into(),
        field_tiers,
        default_tier: AccessTier::Public,
    }
}

/// A Confidential field nested inside an otherwise-public object must NOT leak.
#[test]
fn nested_confidential_field_is_redacted() {
    let policy = policy_with("jwsSignature", AccessTier::Confidential);
    let data = json!({
        "sectorData": { "ok": 1, "jwsSignature": "leak-me" }
    });
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert_eq!(decision.filtered_data["sectorData"]["ok"], json!(1));
    assert!(
        decision.filtered_data["sectorData"]
            .get("jwsSignature")
            .is_none(),
        "nested confidential field must be redacted, got {}",
        decision.filtered_data
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"sectorData.jwsSignature".to_owned())
    );
}

/// A confidential field inside an array of objects is redacted per element.
#[test]
fn confidential_field_in_array_is_redacted() {
    let policy = policy_with("secret", AccessTier::Confidential);
    let data = json!({ "items": [ {"id": 1, "secret": "x"}, {"id": 2, "secret": "y"} ] });
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    for el in decision.filtered_data["items"].as_array().unwrap() {
        assert!(
            el.get("secret").is_none(),
            "array-nested secret must be redacted"
        );
        assert!(el.get("id").is_some());
    }
    assert!(
        decision
            .redacted_fields
            .contains(&"items[0].secret".to_owned())
    );
}

/// Casing/separator drift must not bypass redaction.
#[test]
fn casing_and_separator_drift_does_not_bypass() {
    let policy = policy_with("disassemblyInstructions", AccessTier::Professional);
    let data = json!({ "disassembly_instructions": "secret", "public": 1 });
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert!(
        decision
            .filtered_data
            .get("disassembly_instructions")
            .is_none(),
        "snake_case payload key must match camelCase policy key"
    );
    assert!(decision.filtered_data.get("public").is_some());
}

/// Fail-closed mode: with `default_tier = Confidential`, an unlisted field is redacted.
#[test]
fn fail_closed_default_tier_redacts_unlisted() {
    let mut policy = policy_with("publicField", AccessTier::Public);
    policy.default_tier = AccessTier::Confidential;
    let data = json!({ "publicField": "ok", "unclassified": "should-not-leak" });
    let decision = filter_by_access_tier(&data, &policy, AccessTier::Public);
    assert_eq!(decision.filtered_data["publicField"], json!("ok"));
    assert!(
        decision.filtered_data.get("unclassified").is_none(),
        "fail-closed: unlisted field must be redacted"
    );
}

/// Locks the current-correct Public view of a passport carrying both a nested
/// `facility` snapshot and a `manufacturer`: under the default passport policy,
/// Annex III facility + operator identity stay public (no requirement to redact).
#[test]
fn passport_default_keeps_facility_and_manufacturer_public() {
    let policy = SectorAccessPolicy::passport_default();
    let data = json!({
        "id": "x",
        "manufacturer": { "name": "GreenCell GmbH", "address": "Berlin, DE" },
        "facility": { "scheme": "gln", "value": "4012345000009",
                      "name": "Default Plant", "country": "DE", "address": "1 Allee, Berlin" },
        "operatorIdentifier": "DE123456789"
    });
    let out = filter_by_access_tier(&data, &policy, AccessTier::Public).filtered_data;
    assert_eq!(out["facility"]["value"], json!("4012345000009"));
    assert_eq!(out["facility"]["address"], json!("1 Allee, Berlin"));
    assert_eq!(out["manufacturer"]["address"], json!("Berlin, DE"));
    assert_eq!(out["operatorIdentifier"], json!("DE123456789"));
}

/// Documents the leaf-key collision (crypto): elevating a *generic* leaf name
/// redacts it in **every** object it appears in — here, gating `address` drops
/// both `facility.address` and `manufacturer.address`. This is why policies must
/// use specific field names, and why `facility.address` cannot be gated in
/// isolation without a path-aware matcher. Guards against a naive future edit.
#[test]
fn generic_leaf_key_collides_across_objects() {
    let mut policy = SectorAccessPolicy::passport_default();
    policy
        .field_tiers
        .insert("address".into(), AccessTier::Professional);
    let data = json!({
        "manufacturer": { "name": "ACME", "address": "Berlin, DE" },
        "facility": { "value": "4012345000009", "address": "1 Allee, Berlin" }
    });
    let out = filter_by_access_tier(&data, &policy, AccessTier::Public).filtered_data;
    assert!(
        out["manufacturer"].get("address").is_none(),
        "collision: gating `address` also drops manufacturer.address"
    );
    assert!(
        out["facility"].get("address").is_none(),
        "collision: gating `address` also drops facility.address"
    );
    // Non-colliding leaves are untouched.
    assert_eq!(out["facility"]["value"], json!("4012345000009"));
}
