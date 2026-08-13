use std::collections::HashMap;

use crate::{Audience, Disclosure, SectorCatalog};
use serde_json::json;

use super::filter::filter_by_audience;
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
    let decision = filter_by_audience(&data, &policy, Audience::Public);

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
    let decision = filter_by_audience(&data, &policy, Audience::LegitimateInterest);

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
    let decision = filter_by_audience(&data, &policy, Audience::Authority);

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
    let decision = filter_by_audience(&data, &policy, Audience::Public);
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
    let policy = SectorAccessPolicy::passport_default();
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
    let back: SectorAccessPolicy = serde_json::from_value(json).unwrap();
    assert_eq!(back.name, "textile-1.2.0");
    assert_eq!(back.sector, "textile");
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
    let decision = filter_by_audience(&data, &policy, Audience::Public);
    assert!(decision.filtered_data.get("durabilityScore").is_none());
    assert!(
        decision
            .redacted_fields
            .contains(&"durabilityScore".to_owned())
    );
}

// ── crypto Gap 6: path-aware, fail-closed redaction ──────────────────────────

fn policy_with(name: &str, class: Disclosure) -> SectorAccessPolicy {
    let mut field_disclosure = HashMap::new();
    field_disclosure.insert(name.to_owned(), class);
    SectorAccessPolicy {
        name: "test".into(),
        sector: "test".into(),
        field_disclosure,
        default_disclosure: Disclosure::Public,
    }
}

/// A Confidential field nested inside an otherwise-public object must NOT leak.
#[test]
fn nested_confidential_field_is_redacted() {
    let policy = policy_with("jwsSignature", Disclosure::Conformity);
    let data = json!({
        "sectorData": { "ok": 1, "jwsSignature": "leak-me" }
    });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
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
    let policy = policy_with("secret", Disclosure::Conformity);
    let data = json!({ "items": [ {"id": 1, "secret": "x"}, {"id": 2, "secret": "y"} ] });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
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
    let policy = policy_with("disassemblyInstructions", Disclosure::Restricted);
    let data = json!({ "disassembly_instructions": "secret", "public": 1 });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
    assert!(
        decision
            .filtered_data
            .get("disassembly_instructions")
            .is_none(),
        "snake_case payload key must match camelCase policy key"
    );
    assert!(decision.filtered_data.get("public").is_some());
}

/// Fail-closed mode: with `default_disclosure = Confidential`, an unlisted field is redacted.
#[test]
fn fail_closed_default_disclosure_redacts_unlisted() {
    let mut policy = policy_with("publicField", Disclosure::Public);
    policy.default_disclosure = Disclosure::Conformity;
    let data = json!({ "publicField": "ok", "unclassified": "should-not-leak" });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
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
    let out = filter_by_audience(&data, &policy, Audience::Public).filtered_data;
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
        .field_disclosure
        .insert("address".into(), Disclosure::Restricted);
    let data = json!({
        "manufacturer": { "name": "ACME", "address": "Berlin, DE" },
        "facility": { "value": "4012345000009", "address": "1 Allee, Berlin" }
    });
    let out = filter_by_audience(&data, &policy, Audience::Public).filtered_data;
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

// ── Art. 77(2) lattice ───────────────────────────────────────────────────────

#[test]
fn reclassifying_one_field_changes_the_served_public_bytes() {
    // ⚠️ This test records a DEFECT, not a guarantee. It passes today and the
    // fix will invert it.
    //
    // The disclosure map has no version axis: `sectors/battery.json` carries one
    // flat map and eight schemaVersions, and the map is read from the
    // compiled-in catalog at *serve* time. Passport signatures, by contrast, are
    // frozen at publish and keyed by disclosure set (`disclosure_key`).
    //
    // So the day a delegated act reclassifies a field — restricted → public is
    // the move these acts make — the public view we serve for an
    // already-published passport gains a field its frozen `public` signature
    // never covered. Verification fails for every affected passport at once, and
    // nothing detects it.
    //
    // Below is that mechanism, in isolation and without crypto: the same data
    // and the same audience produce different bytes under two maps that differ
    // by one entry. A signature is a commitment to bytes, so bytes that move
    // under us are the whole defect.
    //
    // The fix is to bind the map (or its hash) to the passport at publish, so
    // the frozen signature and the filter that produced it stay together. When
    // that lands, this test should assert that a passport signed under one map
    // is *refused* rather than silently re-filtered under another.
    let before = battery_policy();
    assert_eq!(
        before.field_disclosure.get("sohMethodology"),
        Some(&Disclosure::Restricted),
        "fixture assumption: sohMethodology is restricted in the shipped catalog"
    );

    let data = json!({
        "gtin": "09506000134352",
        "sohMethodology": "IEC 62660-1:2018",
    });

    let served_before = filter_by_audience(&data, &before, Audience::Public).filtered_data;
    assert!(
        served_before.get("sohMethodology").is_none(),
        "restricted today, so the public view must not carry it"
    );

    // One delegated act later: the same field is public.
    let mut after = battery_policy();
    after
        .field_disclosure
        .insert("sohMethodology".into(), Disclosure::Public);
    let served_after = filter_by_audience(&data, &after, Audience::Public).filtered_data;

    assert_eq!(
        served_after.get("sohMethodology"),
        Some(&json!("IEC 62660-1:2018")),
        "reclassified to public, so the public view must now carry it"
    );

    // The defect, stated as an assertion: same passport, same audience, same
    // disclosure-set key — different bytes. Nothing in the passport records
    // which of these two maps its frozen signature was taken over.
    assert_ne!(
        serde_json::to_string(&served_before).unwrap(),
        serde_json::to_string(&served_after).unwrap(),
        "if these were equal the hazard would not exist and the fix would be unnecessary"
    );
}

#[test]
fn the_annex_xiii_point_4_tier_is_withheld_through_the_real_catalog_policy() {
    // Not a hand-built policy: this reads sectors/battery.json, so it fails if
    // a point-4 field is added to the type and its disclosure entry is
    // forgotten — which would publish measured, per-battery data to anyone
    // scanning the QR code.
    let policy = battery_policy();
    let data = json!({
        "gtin": "09506000134352",
        "dynamicPerformance": { "ratedCapacityAh": 92.0, "capacityFadePct": 8.0 },
        "batteryStatus": "repurposed",
        "usageHistory": { "chargeDischargeCycles": 412 },
    });

    let point_4 = ["dynamicPerformance", "batteryStatus", "usageHistory"];

    for audience in [Audience::Public, Audience::Authority] {
        let view = filter_by_audience(&data, &policy, audience);
        for key in point_4 {
            assert!(
                view.filtered_data.get(key).is_none(),
                "{audience:?} can see '{key}', which Annex XIII point 4 reserves \
                 to holders of a legitimate interest"
            );
        }
        assert!(
            view.filtered_data.get("gtin").is_some(),
            "{audience:?} lost a public field"
        );
    }

    let holder = filter_by_audience(&data, &policy, Audience::LegitimateInterest);
    for key in point_4 {
        assert!(
            holder.filtered_data.get(key).is_some(),
            "a legitimate-interest holder cannot see '{key}', which point 4 grants them"
        );
    }
}

#[test]
fn individual_item_data_is_withheld_from_authorities() {
    // The end-to-end consequence of the lattice, through the real filter: an
    // authority holds Annex XIII points 2 and 3, a legitimate-interest holder
    // holds 2 and 4. Neither sees everything.
    let mut field_disclosure = HashMap::new();
    field_disclosure.insert("dismantlingInfo".into(), Disclosure::Restricted);
    field_disclosure.insert("testReport".into(), Disclosure::Conformity);
    field_disclosure.insert("cycleHistory".into(), Disclosure::Individual);
    let policy = SectorAccessPolicy {
        name: "lattice-test".into(),
        sector: "battery".into(),
        field_disclosure,
        default_disclosure: Disclosure::Public,
    };
    let data = json!({
        "productName": "Cell",
        "dismantlingInfo": "…",
        "testReport": "…",
        "cycleHistory": [1, 2, 3],
    });

    let authority = filter_by_audience(&data, &policy, Audience::Authority);
    assert!(authority.filtered_data.get("testReport").is_some());
    assert!(
        authority.filtered_data.get("cycleHistory").is_none(),
        "Art. 77(2)(b) does not grant authorities Annex XIII point 4"
    );

    let interest = filter_by_audience(&data, &policy, Audience::LegitimateInterest);
    assert!(interest.filtered_data.get("cycleHistory").is_some());
    assert!(
        interest.filtered_data.get("testReport").is_none(),
        "Art. 77(2)(c) does not grant legitimate interest Annex XIII point 3"
    );

    // Both see point 2; neither audience is a superset of the other.
    assert!(authority.filtered_data.get("dismantlingInfo").is_some());
    assert!(interest.filtered_data.get("dismantlingInfo").is_some());

    let public = filter_by_audience(&data, &policy, Audience::Public);
    assert_eq!(public.filtered_data.as_object().unwrap().len(), 1);
    assert!(public.filtered_data.get("productName").is_some());
}
