use std::collections::HashMap;

use crate::{Audience, Disclosure, SectorCatalog};
use serde_json::json;

use super::filter::filter_by_audience;
use super::policy::SectorAccessPolicy;

/// The policy for a sector's current schema version — the path a served
/// passport actually takes.
fn current_policy(sector: &str) -> SectorAccessPolicy {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let (version, _) = reg.latest(sector).expect("sector has a schema");
    SectorAccessPolicy::for_schema_version(sector, &version.to_string())
        .expect("current version yields a policy")
}

fn textile_policy() -> SectorAccessPolicy {
    current_policy("textile")
}

fn battery_policy() -> SectorAccessPolicy {
    current_policy("battery")
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
    let decision = filter_by_audience(&data, &policy, Audience::Public);
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

/// Every property of the current battery schema declares an `x-disclosure`
/// class — **at every depth**.
///
/// This is the guarantee the annotation exists to provide. The catalog map it
/// replaces has no equivalent: a field added there without an entry silently
/// defaults to `Public`, and for Annex XIII point 2, 3 or 4 content that is a
/// leak rather than an omission. Co-locating the class with the property makes
/// the gap visible in the same diff; this test makes it fail the build.
///
/// **A misspelt class is checked too, not just a missing one.**
/// `SectorAccessPolicy::from_schema` matches the four known tokens and drops
/// anything else, so `"restrcted"` produces no map entry and the field falls to
/// `default_disclosure` — `Public`. A typo in one character therefore publishes
/// a restricted field, and asserting only that *some* string is present would
/// not catch it.
///
/// **Depth is the part this used to miss.** It walked `schema["properties"]`
/// and stopped, so 184 properties nested inside objects, array `items` and
/// `definitions` blocks were unchecked — and the constructor could not have
/// read them anyway. Both halves are fixed together: reading one without
/// gating the other leaves a field that silently defaults to public, and
/// gating one without reading the other rejects schemas the code then ignores.
#[test]
fn every_property_declares_a_valid_disclosure_class() {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let (version, json) = reg.latest("battery").expect("battery schema exists");
    let schema: serde_json::Value = serde_json::from_str(json).expect("valid JSON");

    let bad = undeclared_properties(&schema);
    assert!(
        bad.is_empty(),
        "battery v{version}: these properties declare no usable x-disclosure class, so they \
         would default to public: {bad:?}"
    );
}

/// The four tokens `SectorAccessPolicy::from_schema` recognises. Anything else
/// is dropped by that constructor and falls through to the public default.
const VALID_DISCLOSURE_TOKENS: [&str; 4] = ["public", "restricted", "conformity", "individual"];

/// Every property in `schema`, at any depth, that declares no usable class.
///
/// Mirrors the traversal `SectorAccessPolicy::from_schema` performs, for the
/// same reason the two are described together: a gate that walks a different
/// tree from the constructor is a gate over a different schema.
fn undeclared_properties(schema: &serde_json::Value) -> Vec<(String, String)> {
    fn walk(node: &serde_json::Value, path: &str, out: &mut Vec<(String, String)>) {
        let Some(object) = node.as_object() else {
            return;
        };
        if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in properties {
                let child = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                let class = prop
                    .get("x-disclosure")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("<missing>");
                if !VALID_DISCLOSURE_TOKENS.contains(&class) {
                    out.push((child.clone(), class.to_owned()));
                }
                walk(prop, &child, out);
            }
        }
        for key in ["items", "additionalProperties"] {
            if let Some(child) = object.get(key) {
                walk(child, &format!("{path}[]"), out);
            }
        }
        for key in ["definitions", "$defs"] {
            if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
                for (name, definition) in block {
                    walk(definition, &format!("{key}.{name}"), out);
                }
            }
        }
        for key in ["allOf", "anyOf", "oneOf"] {
            if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
                for branch in branches {
                    walk(branch, path, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(schema, "", &mut out);
    out
}

/// No schema declares one field name in two different classes.
///
/// Matching is by normalized **leaf name** at any depth, so `materialComposition.name`
/// and `criticalRawMaterial.name` are one key. A schema declaring them
/// differently is asking for something the matcher cannot express, and the
/// constructor would have to pick — silently, and for every field of that name
/// in the document.
///
/// It picks the more restrictive one, which is the safe direction but is still
/// a guess at what the author meant. This test means no shipped schema ever
/// makes it guess. It is also what keeps `disclosure_for_field` honest: the
/// tie-break exists for hand-built policies, not for these.
///
/// **This is the constraint that decides how a nested field may be classified.**
/// A nested property whose leaf name is shared with a more permissive field
/// elsewhere in the same schema cannot be restricted by annotation alone —
/// restricting it would restrict the twin. Where that arises today the parent
/// object carries the restriction instead, and the filter removes the whole
/// subtree before ever reaching the leaf. Expressing it on the leaf itself
/// needs a path-aware matcher, which this crate does not have.
#[test]
fn no_schema_declares_one_field_name_in_two_classes() {
    use std::collections::HashMap;

    let reg = crate::schemas::VersionedSchemaRegistry::new();
    for sector in reg.sectors() {
        for version in reg.versions_for(sector) {
            let json = reg.get(sector, version).expect("registry listed it");
            let schema: serde_json::Value = serde_json::from_str(json).expect("valid JSON");

            let mut seen: HashMap<String, (String, Disclosure)> = HashMap::new();
            let mut clashes: Vec<String> = Vec::new();
            collect_declared(&schema, "", &mut |path: &str, name: &str, class| {
                let key: String = name
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .map(|c| c.to_ascii_lowercase())
                    .collect();
                if let Some((first_path, first)) = seen.get(&key) {
                    if *first != class {
                        clashes.push(format!(
                            "'{name}' is {first:?} at {first_path} and {class:?} at {path}"
                        ));
                    }
                } else {
                    seen.insert(key, (path.to_owned(), class));
                }
            });

            assert!(
                clashes.is_empty(),
                "{sector} v{version}: one field name declared in two classes, so the policy \
                 cannot express both: {clashes:?}"
            );
        }
    }
}

/// Visit every declared `(path, name, class)` triple in a schema.
fn collect_declared(
    node: &serde_json::Value,
    path: &str,
    visit: &mut impl FnMut(&str, &str, Disclosure),
) {
    let Some(object) = node.as_object() else {
        return;
    };
    if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            let child = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if let Some(class) = prop
                .get("x-disclosure")
                .and_then(serde_json::Value::as_str)
                .and_then(|t| match t {
                    "public" => Some(Disclosure::Public),
                    "restricted" => Some(Disclosure::Restricted),
                    "conformity" => Some(Disclosure::Conformity),
                    "individual" => Some(Disclosure::Individual),
                    _ => None,
                })
            {
                visit(&child, name, class);
            }
            collect_declared(prop, &child, visit);
        }
    }
    for key in ["items", "additionalProperties"] {
        if let Some(child) = object.get(key) {
            collect_declared(child, &format!("{path}[]"), visit);
        }
    }
    for key in ["definitions", "$defs"] {
        if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
            for (name, definition) in block {
                collect_declared(definition, &format!("{key}.{name}"), visit);
            }
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
            for branch in branches {
                collect_declared(branch, path, visit);
            }
        }
    }
}

/// Every schema version of every sector yields a policy, and every property in
/// each declares a class.
///
/// The backfill guarantee. `for_schema_version` fails closed, so a schema
/// version left un-annotated would not misbehave — it would refuse to serve
/// that passport at all. Older versions carry today's classes because no
/// passport has ever been published under any of them; there is no historical
/// map to preserve, and this is the last moment that is true.
#[test]
fn every_sector_version_yields_a_fully_classified_policy() {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let mut checked = 0usize;
    for sector in reg.sectors() {
        for version in reg.versions_for(sector) {
            let json = reg.get(sector, version).expect("registry listed it");
            let schema: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
            let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) else {
                continue;
            };
            // Missing *and* misspelt: both land the field on the public
            // default, so both have to fail the build. Walked at every depth —
            // a nested property with no class defaults to public exactly as a
            // top-level one does.
            let _ = properties;
            let undeclared = undeclared_properties(&schema);
            assert!(
                undeclared.is_empty(),
                "{sector} v{version}: unclassified properties {undeclared:?}"
            );
            assert!(
                SectorAccessPolicy::for_schema_version(sector, &version.to_string()).is_some(),
                "{sector} v{version} yields no policy"
            );
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} schema versions walked");
}

/// The schema-sourced policy agrees with the catalog map it will replace.
///
/// Until the cutover both exist, and a silent divergence between them would be
/// worse than either alone — the served view would depend on which constructor
/// a caller happened to use.
#[test]
#[expect(
    deprecated,
    reason = "this test exists precisely to compare the two constructors"
)]
fn the_schema_policy_matches_the_catalog_policy_today() {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let (version, json) = reg.latest("battery").expect("battery schema exists");
    let from_schema = SectorAccessPolicy::from_schema("battery", &version.to_string(), json)
        .expect("the current schema yields a policy");
    let from_catalog =
        SectorAccessPolicy::from_catalog(&SectorCatalog::new(), "battery").expect("in catalog");

    for (field, class) in &from_catalog.field_disclosure {
        assert_eq!(
            from_schema.disclosure_for_field(field),
            *class,
            "'{field}' disagrees between the catalog map and the schema annotation"
        );
    }
}

/// A schema with no `properties` yields no policy rather than an all-public one.
///
/// Failing open here would serve every field of an unparseable sector to
/// anyone, which is the fail-open class this project already fixed once at the
/// unknown-sector boundary.
#[test]
fn an_unusable_schema_yields_no_policy() {
    assert!(SectorAccessPolicy::from_schema("battery", "9.9.9", "not json").is_none());
    assert!(SectorAccessPolicy::from_schema("battery", "9.9.9", r#"{"type":"object"}"#).is_none());
}

/// An unrecognised `x-disclosure` token fails **open**, and this pins that.
///
/// `from_schema` matches the four known tokens and drops anything else, so a
/// misspelt class produces no map entry and the field falls to
/// `default_disclosure` — `Public`. One transposed character therefore publishes
/// a field meant to be withheld, and the failure is silent at every layer that
/// consumes the policy.
///
/// Raising the default instead does not work: the policy is applied to the whole
/// document and the passport envelope's public fields are declared in no sector
/// schema, so a non-public default would erase them. The containment is
/// therefore build-time —
/// [`every_property_declares_a_valid_disclosure_class`] and its all-sector
/// counterpart reject a schema carrying a token this constructor cannot read.
///
/// This test exists so that behaviour is *recorded* rather than assumed. If
/// `from_schema` is ever changed to reject an unknown token outright, this test
/// is the one that should fail and be rewritten to assert the refusal.
#[test]
fn an_unrecognised_disclosure_token_falls_through_to_public() {
    let schema = r#"{
        "properties": {
            "safetyMeasures": { "type": "string", "x-disclosure": "restrcted" },
            "cathodeMaterial": { "type": "string", "x-disclosure": "restricted" }
        }
    }"#;
    let policy =
        SectorAccessPolicy::from_schema("battery", "9.9.9", schema).expect("has properties");

    assert!(
        !policy.field_disclosure.contains_key("safetyMeasures"),
        "a misspelt token must not produce a map entry"
    );
    assert_eq!(
        policy.disclosure_for_field("safetyMeasures"),
        Disclosure::Public,
        "and the field therefore falls to the public default — the hazard"
    );
    // The correctly spelt sibling is unaffected, so this is a per-field silent
    // failure rather than a whole-policy one, which is what makes it easy to miss.
    assert_eq!(
        policy.disclosure_for_field("cathodeMaterial"),
        Disclosure::Restricted
    );

    // And the leak is real through the filter, not just the map.
    let data = json!({ "safetyMeasures": "Isolate at the service disconnect" });
    let served = filter_by_audience(&data, &policy, Audience::Public).filtered_data;
    assert!(
        served.get("safetyMeasures").is_some(),
        "the public view carries a field the typo declassified"
    );
}

/// A nested property's declared class is read, and it is enforced.
///
/// The regression this pins: `from_schema` read `schema["properties"]` and
/// stopped, while `filter_by_audience` classifies keys at every depth. A
/// property annotated `restricted` inside an object was not in the map at all,
/// so it fell to `default_disclosure` — `Public` — and was served to anyone.
///
/// The shape of that failure is what makes it worth a test of its own: the
/// author did everything right. The annotation sat in the place the
/// constructor's own doc comment says to put it, and it did nothing. Neither
/// build-time gate looked deep enough to say so.
#[test]
fn a_nested_property_is_classified_by_its_own_annotation() {
    let schema = r#"{
        "properties": {
            "supplier": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "tradingName":     { "type": "string", "x-disclosure": "public" },
                    "internalContact": { "type": "string", "x-disclosure": "restricted" }
                }
            },
            "shipments": {
                "type": "array",
                "x-disclosure": "public",
                "items": {
                    "type": "object",
                    "properties": {
                        "reference":  { "type": "string", "x-disclosure": "public" },
                        "unitCostEur": { "type": "number", "x-disclosure": "conformity" }
                    }
                }
            }
        }
    }"#;
    let policy =
        SectorAccessPolicy::from_schema("battery", "9.9.9", schema).expect("has properties");

    assert_eq!(
        policy.disclosure_for_field("internalContact"),
        Disclosure::Restricted,
        "a nested annotation must reach the policy map"
    );
    assert_eq!(
        policy.disclosure_for_field("unitCostEur"),
        Disclosure::Conformity,
        "inside array items, too"
    );

    // And through the filter, which is where it actually mattered.
    let data = json!({
        "supplier":  { "tradingName": "ACME", "internalContact": "ops@example.invalid" },
        "shipments": [ { "reference": "S-1", "unitCostEur": 12.5 } ]
    });
    let public = filter_by_audience(&data, &policy, Audience::Public).filtered_data;

    assert!(
        public["supplier"].get("internalContact").is_none(),
        "the public view must not carry a nested restricted field"
    );
    assert_eq!(
        public["supplier"]["tradingName"],
        json!("ACME"),
        "its public sibling survives"
    );
    assert!(
        public["shipments"][0].get("unitCostEur").is_none(),
        "conformity content inside an array item is withheld from the public"
    );
    assert_eq!(public["shipments"][0]["reference"], json!("S-1"));

    // The audience that may see it, does.
    let authority = filter_by_audience(&data, &policy, Audience::Authority).filtered_data;
    assert_eq!(authority["shipments"][0]["unitCostEur"], json!(12.5));
    assert_eq!(
        authority["supplier"]["internalContact"],
        json!("ops@example.invalid")
    );
}

/// Two normalized-equal keys give one answer, and the same answer every time.
///
/// `field_disclosure` is keyed by literal name but matched after
/// normalization, so `jwsSignature` and `jws_signature` both answer one lookup.
/// Resolving by `HashMap` iteration order meant resolving by an unspecified,
/// per-map-reseeded order: the same policy could answer `Conformity` once and
/// `Public` the next call, in one process.
///
/// Built fresh inside the loop on purpose — `RandomState` seeds per map, so a
/// single map reused across iterations would report a stable answer and prove
/// nothing.
#[test]
fn an_ambiguous_field_name_resolves_the_same_way_every_time() {
    // A `HashSet`, not a `BTreeSet`: `Disclosure` is a lattice and deliberately
    // implements no `Ord`, because ordering it is exactly the mistake
    // `Audience`'s doc comment exists to prevent.
    let mut answers = std::collections::HashSet::new();
    for _ in 0..512 {
        let mut policy = SectorAccessPolicy::passport_default();
        policy
            .field_disclosure
            .insert("jws_signature".into(), Disclosure::Public);
        answers.insert(policy.disclosure_for_field("jwsSignature"));
    }
    assert_eq!(
        answers.len(),
        1,
        "one lookup must not depend on hash order; got {answers:?}"
    );
    assert!(
        answers.contains(&Disclosure::Conformity),
        "and ambiguity must resolve to the more restrictive class; got {answers:?}"
    );
}

/// The version axis is live: two versions of one sector yield different maps.
///
/// Without this, `for_schema_version` could be reading the current version for
/// every input and nothing would notice — every version currently carries
/// identical *classes*, so class comparison cannot tell the two apart. The
/// field **set** can: `dueDiligenceUrl` arrived in battery v2.0.0, so v1.0.0
/// must not classify it.
///
/// A field the declared version does not know is not thereby restricted — it
/// falls to the public default, which is exactly why `dpp-aas` carries a
/// structural backstop that drops `sectorData` keys the version does not
/// declare. Both halves are asserted here so the pair cannot drift apart.
#[test]
fn an_older_version_classifies_only_the_fields_it_declared() {
    let old = SectorAccessPolicy::for_schema_version("battery", "1.0.0")
        .expect("v1.0.0 is a registered battery version");
    let current = battery_policy();

    assert!(
        !old.field_disclosure.contains_key("dueDiligenceUrl"),
        "v1.0.0 predates the field and must not classify it"
    );
    assert!(
        current.field_disclosure.contains_key("dueDiligenceUrl"),
        "the current version does classify it"
    );
    assert!(
        old.field_disclosure.len() < current.field_disclosure.len(),
        "v1.0.0 declares strictly fewer fields, so the two policies are not the same object"
    );
    // The consequence the backstop exists for.
    assert_eq!(
        old.disclosure_for_field("dueDiligenceUrl"),
        Disclosure::Public,
        "an undeclared field falls to the public default rather than being withheld"
    );
}

/// Why disclosure is sourced from the passport's own schema version.
///
/// This began as a record of a live defect: the class map had no version axis,
/// so it was read from the compiled-in catalog at *serve* time while passport
/// signatures were frozen at publish and keyed by disclosure set. A delegated
/// act reclassifying one field — `restricted → public` is the move these acts
/// make — would have changed the public bytes served for an already-published
/// passport, breaking verification for every affected passport at once with
/// nothing to detect it.
///
/// That is fixed: `SectorAccessPolicy::for_schema_version` reads the classes
/// from the schema version a passport declares, so a passport carries its
/// classification with it and a newer version cannot move its bytes.
///
/// The test is kept, pointed at the mechanism rather than the defect, because
/// the hazard is what justifies the design. It shows the thing the version axis
/// prevents: one entry's difference is enough to move the served bytes, and a
/// signature is a commitment to bytes.
///
/// **It is not a regression guard for the fix, and cannot be one yet.** Every
/// battery schema version currently carries identical classes — deliberately,
/// since no passport has been published under any of them — so no two versions
/// disagree about anything and no test can distinguish "use the passport's
/// version" from "use the current version". The first genuine divergence will
/// be the first reclassification made after a passport exists, and that is the
/// point at which this test should be replaced by one that pins an old
/// passport's bytes against a newer version's map.
#[test]
fn one_reclassified_field_is_enough_to_move_the_served_bytes() {
    let before = battery_policy();
    assert_eq!(
        before.field_disclosure.get("sohMethodology"),
        Some(&Disclosure::Restricted),
        "fixture assumption: sohMethodology is restricted in the current schema"
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

    // The hazard, stated as an assertion: same data, same audience, same
    // disclosure-set key — different bytes, from one entry's difference. This
    // is why the class map must travel with the passport rather than be looked
    // up fresh at serve time.
    assert_ne!(
        serde_json::to_string(&served_before).unwrap(),
        serde_json::to_string(&served_after).unwrap(),
        "if these were equal the version axis would buy nothing"
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

/// Annex XIII points 2 and 3 land in different audiences, and the lattice is
/// not a ladder.
///
/// Point 2 reaches **both** non-public audiences; point 3 reaches authorities
/// **only**; point 4 reaches legitimate interest **only** and explicitly not
/// authorities. No integer ordering expresses that, which is why `Audience` is
/// not `Ord` — and why this reads the shipped catalog rather than a hand-built
/// policy, so a missing disclosure entry fails here instead of leaking.
#[test]
fn points_two_three_and_four_each_reach_a_different_audience() {
    let policy = battery_policy();
    let data = json!({
        "gtin": "09506000134352",
        "safetyMeasures": "Do not puncture",
        "testReportResults": "Report 42: pass",
        "batteryStatus": "original",
    });

    let public = filter_by_audience(&data, &policy, Audience::Public).filtered_data;
    for key in ["safetyMeasures", "testReportResults", "batteryStatus"] {
        assert!(
            public.get(key).is_none(),
            "the public view carries '{key}', which is not point 1 content"
        );
    }

    let authority = filter_by_audience(&data, &policy, Audience::Authority).filtered_data;
    assert!(authority.get("safetyMeasures").is_some(), "point 2");
    assert!(authority.get("testReportResults").is_some(), "point 3");
    assert!(
        authority.get("batteryStatus").is_none(),
        "point 4 is withheld from authorities — Art. 77(2)(b) does not reach it"
    );

    let holder = filter_by_audience(&data, &policy, Audience::LegitimateInterest).filtered_data;
    assert!(holder.get("safetyMeasures").is_some(), "point 2");
    assert!(holder.get("batteryStatus").is_some(), "point 4");
    assert!(
        holder.get("testReportResults").is_none(),
        "point 3 is authorities only — a legitimate interest does not reach test reports"
    );
}
