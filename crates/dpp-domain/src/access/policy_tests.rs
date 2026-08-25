//! Deriving a policy from a schema: unusable input, unrecognised tokens, nested
//! annotations, and what a version bump moves.

use serde_json::json;

use crate::{Audience, Disclosure, ProductGroupCatalog};

use super::policy::ProductGroupAccessPolicy;
use super::tests::{battery_policy, filter_payload};

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
    let from_schema = ProductGroupAccessPolicy::from_schema("battery", &version.to_string(), json)
        .expect("the current schema yields a policy");
    let from_catalog =
        ProductGroupAccessPolicy::from_catalog(&ProductGroupCatalog::new(), "battery")
            .expect("in catalog");

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
/// Failing open here would serve every field of an unparseable product group to
/// anyone, which is the fail-open class this project already fixed once at the
/// unknown-product group boundary.
#[test]
fn an_unusable_schema_yields_no_policy() {
    assert!(ProductGroupAccessPolicy::from_schema("battery", "9.9.9", "not json").is_none());
    assert!(
        ProductGroupAccessPolicy::from_schema("battery", "9.9.9", r#"{"type":"object"}"#).is_none()
    );
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
/// document and the passport envelope's public fields are declared in no product group
/// schema, so a non-public default would erase them. The containment is
/// therefore build-time —
/// [`every_property_declares_a_valid_disclosure_class`] and its all-product group
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
        ProductGroupAccessPolicy::from_schema("battery", "9.9.9", schema).expect("has properties");

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
    let served = filter_payload(&data, &policy, Audience::Public).filtered_data;
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
        ProductGroupAccessPolicy::from_schema("battery", "9.9.9", schema).expect("has properties");

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
    let public = filter_payload(&data, &policy, Audience::Public).filtered_data;

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
    let authority = filter_payload(&data, &policy, Audience::Authority).filtered_data;
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
        let mut policy = ProductGroupAccessPolicy::passport_default();
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

/// The version axis is live: two versions of one product group yield different maps.
///
/// Without this, `for_schema_version` could be reading the current version for
/// every input and nothing would notice — every version currently carries
/// identical *classes*, so class comparison cannot tell the two apart. The
/// field **set** can: `dueDiligenceUrl` arrived in battery v2.0.0, so v1.0.0
/// must not classify it.
///
/// A field the declared version does not know is not thereby restricted — it
/// falls to the public default, which is exactly why `dpp-aas` carries a
/// structural backstop that drops `productGroupData` keys the version does not
/// declare. Both halves are asserted here so the pair cannot drift apart.
#[test]
fn an_older_version_classifies_only_the_fields_it_declared() {
    let old = ProductGroupAccessPolicy::for_schema_version("battery", "1.0.0")
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
/// That is fixed: `ProductGroupAccessPolicy::for_schema_version` reads the classes
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

    let served_before = filter_payload(&data, &before, Audience::Public).filtered_data;
    assert!(
        served_before.get("sohMethodology").is_none(),
        "restricted today, so the public view must not carry it"
    );

    // One delegated act later: the same field is public.
    let mut after = battery_policy();
    after
        .field_disclosure
        .insert("sohMethodology".into(), Disclosure::Public);
    let served_after = filter_payload(&data, &after, Audience::Public).filtered_data;

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
