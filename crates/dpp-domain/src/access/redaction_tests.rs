//! Path-aware, fail-closed redaction: nesting, arrays, key-name drift, and the
//! default that applies to a field no schema classified.

use std::collections::HashMap;

use serde_json::json;

use crate::{Audience, Disclosure};

use super::filter::filter_by_audience;
use super::policy::ProductGroupAccessPolicy;
use super::tests::filter_payload;

/// A policy classifying one **product-group** field — the scope a schema's
/// `x-disclosure` annotations live in. Tests using it put their data under
/// `productGroupData`, where such a class applies.
fn policy_with(name: &str, class: Disclosure) -> ProductGroupAccessPolicy {
    let mut field_disclosure = HashMap::new();
    field_disclosure.insert(name.to_owned(), class);
    ProductGroupAccessPolicy {
        name: "test".into(),
        product_group: "test".into(),
        field_disclosure,
        envelope_disclosure: HashMap::new(),
        default_disclosure: Disclosure::Public,
    }
}

/// A policy classifying one **envelope** field — the scope conformity evidence
/// and passport-level classes live in, which applies at any depth.
fn envelope_policy_with(name: &str, class: Disclosure) -> ProductGroupAccessPolicy {
    let mut envelope_disclosure = HashMap::new();
    envelope_disclosure.insert(name.to_owned(), class);
    ProductGroupAccessPolicy {
        name: "test".into(),
        product_group: "test".into(),
        field_disclosure: HashMap::new(),
        envelope_disclosure,
        default_disclosure: Disclosure::Public,
    }
}

/// A Confidential field nested inside an otherwise-public object must NOT leak.
#[test]
fn nested_confidential_field_is_redacted() {
    let policy = policy_with("jwsSignature", Disclosure::Conformity);
    let data = json!({
        "productGroupData": { "ok": 1, "jwsSignature": "leak-me" }
    });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
    assert_eq!(decision.filtered_data["productGroupData"]["ok"], json!(1));
    assert!(
        decision.filtered_data["productGroupData"]
            .get("jwsSignature")
            .is_none(),
        "nested confidential field must be redacted, got {}",
        decision.filtered_data
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"productGroupData.jwsSignature".to_owned())
    );
}

/// A confidential field inside an array of objects is redacted per element.
#[test]
fn confidential_field_in_array_is_redacted() {
    let policy = policy_with("secret", Disclosure::Conformity);
    let data = json!({
        "productGroupData": { "items": [ {"id": 1, "secret": "x"}, {"id": 2, "secret": "y"} ] }
    });
    let decision = filter_by_audience(&data, &policy, Audience::Public);
    for el in decision.filtered_data["productGroupData"]["items"]
        .as_array()
        .unwrap()
    {
        assert!(
            el.get("secret").is_none(),
            "array-nested secret must be redacted"
        );
        assert!(el.get("id").is_some());
    }
    assert!(
        decision
            .redacted_fields
            .contains(&"productGroupData.items[0].secret".to_owned())
    );
}

/// Casing/separator drift must not bypass redaction.
#[test]
fn casing_and_separator_drift_does_not_bypass() {
    let policy = policy_with("disassemblyInstructions", Disclosure::Restricted);
    let data = json!({ "disassembly_instructions": "secret", "public": 1 });
    let decision = filter_payload(&data, &policy, Audience::Public);
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
    let mut policy = envelope_policy_with("publicField", Disclosure::Public);
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
    let policy = ProductGroupAccessPolicy::passport_default();
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

/// Documents what remains of the leaf-key collision, and where it now stops.
///
/// **Still true, within one scope:** elevating a *generic* leaf name redacts it
/// in every object it appears in at that scope. Gating `address` drops both
/// `facility.address` and `manufacturer.address`, because matching is by leaf
/// name and neither is more specific than the other. Policies must therefore
/// still use specific field names.
///
/// **No longer true across scopes:** a class drawn from a *product group's*
/// schema no longer reaches the envelope. It used to, and that was the defect —
/// a schema describes the contents of `productGroupData` and has no authority
/// over the passport around it, so a product group declaring `address`,
/// `recordedAt` or `name` could silently reclassify an envelope field of the
/// same name. Both halves are asserted below; dropping either loses a real
/// property.
#[test]
fn generic_leaf_key_collides_within_a_scope_but_not_across_them() {
    let data = json!({
        "manufacturer": { "name": "ACME", "address": "Berlin, DE" },
        "facility": { "value": "4012345000009", "address": "1 Allee, Berlin" },
        "productGroupData": { "supplier": { "address": "Rue X, Paris" } }
    });

    // Envelope-scoped class: collides across envelope objects, as before.
    let mut envelope = ProductGroupAccessPolicy::passport_default();
    envelope
        .envelope_disclosure
        .insert("address".into(), Disclosure::Restricted);
    let out = filter_by_audience(&data, &envelope, Audience::Public).filtered_data;
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

    // Product-group-scoped class: reaches its own payload and nothing else.
    let product_group = policy_with("address", Disclosure::Restricted);
    let out = filter_by_audience(&data, &product_group, Audience::Public).filtered_data;
    assert!(
        out["productGroupData"]["supplier"].get("address").is_none(),
        "a product group's class must apply inside its own payload"
    );
    assert_eq!(
        out["manufacturer"]["address"],
        json!("Berlin, DE"),
        "a product group's schema has no authority over the envelope"
    );
    assert_eq!(out["facility"]["address"], json!("1 Allee, Berlin"));
}
