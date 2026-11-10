//! Schema-driven policy construction, pinned: the rules a schema must satisfy to
//! yield a policy at all, and the conservative reading of conditional, dotted,
//! conflicting and budget-exceeding shapes. Split from `policy_tests` by size.

use serde_json::json;

use crate::{Audience, Disclosure};

use super::policy::{DocumentScope, ProductGroupAccessPolicy};
use super::tests::battery_policy;

/// A restriction declared inside `then` is enforced, although the branch may not
/// apply: the walk collects conditional rules unconditionally, which is the safe
/// error. Before, the branch was not read and the field fell to `Public`.
#[test]
fn a_restriction_inside_a_conditional_branch_is_enforced() {
    let schema = json!({
        "properties": { "name": { "type": "string" } },
        "if": { "properties": { "name": { "const": "x" } } },
        "then": { "properties": {
            "safetyMeasures": { "type": "string", "x-disclosure": "restricted" }
        } },
    });
    let policy = ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema.to_string())
        .expect("has properties");
    assert_eq!(
        policy.disclosure_for_field("safetyMeasures"),
        Disclosure::Restricted
    );
}

/// A property name containing a dot cannot be expressed as a policy path: the
/// dot reads as a segment boundary, and the property's own rule would never
/// match it. Such a schema yields no policy rather than a partial one.
#[test]
fn a_dotted_property_name_refuses_the_whole_schema() {
    let schema = json!({
        "properties": {
            "supplier.secret": { "type": "string", "x-disclosure": "restricted" },
            "name": { "type": "string" },
        },
    });
    assert!(
        ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema.to_string()).is_none()
    );
}

/// The schemas are draft-07, so its own spellings of the unread keywords, and the
/// tuple form of `items`, refuse a schema that hides an annotation in them.
#[test]
fn an_annotation_under_a_draft_07_keyword_the_walk_does_not_read_refuses_the_schema() {
    let hidden = json!({ "type": "string", "x-disclosure": "restricted" });
    for schema in [
        json!({ "properties": { "a": { "type": "object" } },
                "dependencies": { "a": { "properties": { "secret": hidden } } } }),
        json!({ "properties": { "a": { "type": "array", "items": [{ "type": "string" }],
                                       "additionalItems": hidden } } }),
        json!({ "properties": { "a": { "type": "array", "items": [hidden] } } }),
    ] {
        assert!(
            ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema.to_string())
                .is_none(),
            "{schema}"
        );
    }
}

/// Conformity (authorities) and Individual (the individual) declared for one path
/// have no class that is safe for both, so the schema is refused.
#[test]
fn conformity_and_individual_declared_for_one_path_refuse_the_schema() {
    let schema = json!({
        "properties": { "x": { "type": "string", "x-disclosure": "conformity" } },
        "allOf": [ { "properties": { "x": { "type": "string", "x-disclosure": "individual" } } } ],
    });
    assert!(
        ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema.to_string()).is_none()
    );
}

/// A reference graph that expands to millions of visits is refused by the walk's
/// budget. Each level below refers to the one beneath it twice, so the walk costs
/// 2^20 visits for a schema of about twenty small definitions.
#[test]
fn a_schema_whose_references_expand_past_the_work_budget_is_refused() {
    let mut definitions = serde_json::Map::new();
    definitions.insert("d0".to_owned(), json!({ "type": "object" }));
    for depth in 1..=20 {
        let below = format!("#/definitions/d{}", depth - 1);
        definitions.insert(
            format!("d{depth}"),
            json!({ "allOf": [ { "$ref": below }, { "$ref": below } ] }),
        );
    }
    let schema = json!({
        "properties": { "x": { "$ref": "#/definitions/d20" } },
        "definitions": definitions,
    });
    assert!(
        ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema.to_string()).is_none()
    );
}

/// A product rule that opens a universal conformity field at a deeper path does
/// not make it visible: the universal class is a floor under every path.
#[test]
fn a_product_rule_cannot_open_a_universal_conformity_field() {
    let schema = json!({
        "properties": { "supplier": { "properties": {
            "jwsSignature": { "type": "string", "x-disclosure": "public" }
        } } },
    });
    let policy = ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema.to_string())
        .expect("has properties");
    assert_eq!(
        policy.disclosure_for_path(
            &["supplier", "jwsSignature"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Conformity
    );
}

/// The whole-passport filter never returns the proofs at the root, whoever asks:
/// each carries a payload that can hold content the audience may not see.
#[test]
fn the_whole_passport_filter_drops_root_proofs_for_every_audience() {
    let policy = battery_policy();
    let data = json!({ "id": "p1", "jwsSignature": "a.b.c" });
    for audience in [
        Audience::Public,
        Audience::Authority,
        Audience::LegitimateInterest,
    ] {
        let served = super::filter::filter_by_audience(&data, &policy, audience).filtered_data;
        assert!(served.get("jwsSignature").is_none(), "{audience:?}");
    }
}

/// A personal-data statement is withheld together with the field it describes.
/// `secret` is conformity evidence, which a legitimate-interest reader may not see,
/// so its statement goes too; `name` is public, so its statement stays.
#[test]
fn a_personal_data_statement_is_withheld_with_the_field_it_describes() {
    let schema = json!({ "properties": {
        "secret": { "type": "string", "x-disclosure": "conformity" },
        "name": { "type": "string" },
    } });
    let policy = ProductGroupAccessPolicy::from_schema("battery", "9.9.9", &schema.to_string())
        .expect("has properties");
    let data = json!({
        "productGroupData": { "secret": "s", "name": "n" },
        "personalData": {
            "secret": { "statement": "about secret" },
            "name": { "statement": "about name" },
        },
    });

    let served = super::filter::filter_by_audience(&data, &policy, Audience::LegitimateInterest)
        .filtered_data;
    let statements = served["personalData"].as_object().expect("statements kept");
    assert!(statements.contains_key("name"));
    assert!(!statements.contains_key("secret"));
}
