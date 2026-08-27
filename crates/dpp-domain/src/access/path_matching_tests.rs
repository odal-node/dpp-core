//! Path-aware disclosure matching: a leaf name may carry different classes in
//! different places, and the most specific statement wins.
//!
//! The property that matters most is the first test here — every class declared
//! in every shipped schema must still resolve to exactly what it declares. The
//! rest exercise the expressiveness that resolving by path unlocks.

use serde_json::json;

use crate::{Audience, Disclosure};

use super::filter::filter_by_audience_in_scope;
use super::policy::{DocumentScope, ProductGroupAccessPolicy};

/// The limit this change removes: one leaf, two classes, two places.
#[test]
fn a_shared_leaf_name_is_classified_per_path() {
    let policy = policy_from(&json!({
        "properties": {
            "materialComposition": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "name": { "type": "string", "x-disclosure": "restricted" }
                }
            },
            "criticalRawMaterials": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "name": { "type": "string", "x-disclosure": "public" }
                }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(
            &["materialComposition", "name"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Restricted,
        "the restricted position must keep its class"
    );
    assert_eq!(
        policy.disclosure_for_path(
            &["criticalRawMaterials", "name"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Public,
        "the public twin must not be dragged restricted with it"
    );
}

/// And the filter acts on it — the same leaf survives under one parent and is
/// removed under the other, in one pass over one document.
#[test]
fn the_filter_redacts_a_shared_leaf_under_only_one_parent() {
    let policy = policy_from(&json!({
        "properties": {
            "materialComposition": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "name": { "type": "string", "x-disclosure": "restricted" }
                }
            },
            "criticalRawMaterials": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "name": { "type": "string", "x-disclosure": "public" }
                }
            }
        }
    }));

    let document = json!({
        "materialComposition": { "name": "lithium" },
        "criticalRawMaterials": { "name": "cobalt" }
    });

    let decision = filter_by_audience_in_scope(
        &document,
        &policy,
        Audience::Public,
        DocumentScope::ProductGroupData,
    );

    assert!(
        decision.filtered_data["materialComposition"]
            .get("name")
            .is_none(),
        "the restricted one must go"
    );
    assert_eq!(
        decision.filtered_data["criticalRawMaterials"]["name"], "cobalt",
        "the public one must stay — over-redacting Annex III content is not the safe direction"
    );
    assert!(
        decision
            .redacted_fields
            .contains(&"materialComposition.name".to_owned()),
        "the redaction should be reported by its path: {:?}",
        decision.redacted_fields
    );
}

/// An array index is not a path segment: an element sits where its key sits.
#[test]
fn an_array_index_is_not_a_path_segment() {
    let policy = policy_from(&json!({
        "properties": {
            "materialComposition": {
                "type": "array",
                "x-disclosure": "public",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "x-disclosure": "restricted" }
                    }
                }
            }
        }
    }));

    let document = json!({
        "materialComposition": [{ "name": "lithium" }, { "name": "cobalt" }]
    });

    let decision = filter_by_audience_in_scope(
        &document,
        &policy,
        Audience::Public,
        DocumentScope::ProductGroupData,
    );

    for i in 0..2 {
        assert!(
            decision.filtered_data["materialComposition"][i]
                .get("name")
                .is_none(),
            "element {i} kept a restricted field, so the index broke the path match"
        );
    }
    assert!(
        decision
            .redacted_fields
            .contains(&"materialComposition[0].name".to_owned()),
        "the report should still carry the index: {:?}",
        decision.redacted_fields
    );
}

/// A one-segment policy key still matches at any depth, so every policy written
/// before paths existed behaves exactly as it did.
#[test]
fn a_bare_leaf_policy_key_still_matches_at_any_depth() {
    let policy = policy_from(&json!({
        "properties": {
            "svhcSubstances": { "type": "string", "x-disclosure": "restricted" }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(&["svhcSubstances"], DocumentScope::ProductGroupData),
        Disclosure::Restricted
    );
    assert_eq!(
        policy.disclosure_for_path(
            &["deeply", "nested", "svhcSubstances"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Restricted,
        "a bare leaf is a one-segment suffix, not a top-level-only rule"
    );
}

/// Specificity ordering: the longer match wins, whichever way it points.
#[test]
fn a_more_specific_path_beats_a_bare_leaf() {
    let mut policy = ProductGroupAccessPolicy::passport_default();
    policy.product_group = "test".into();
    policy
        .field_disclosure
        .insert("value".into(), Disclosure::Restricted);
    policy
        .field_disclosure
        .insert("public_block.value".into(), Disclosure::Public);

    assert_eq!(
        policy.disclosure_for_path(&["anywhere", "value"], DocumentScope::ProductGroupData),
        Disclosure::Restricted,
        "the bare leaf governs where nothing more specific applies"
    );
    assert_eq!(
        policy.disclosure_for_path(&["public_block", "value"], DocumentScope::ProductGroupData),
        Disclosure::Public,
        "the two-segment key is the more precise statement and must win"
    );
}

/// Casing and separator drift must not defeat a path match, exactly as it does
/// not defeat a leaf match.
#[test]
fn path_matching_is_case_and_separator_insensitive() {
    let mut policy = ProductGroupAccessPolicy::passport_default();
    policy.field_disclosure.insert(
        "material_composition.cas_number".into(),
        Disclosure::Restricted,
    );

    assert_eq!(
        policy.disclosure_for_path(
            &["materialComposition", "casNumber"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Restricted,
        "a policy written in snake_case must still govern a camelCase document"
    );
}

/// **The fail-closed fallback.** A caller that knows only a leaf name cannot
/// supply the path, and without a fallback the lookup would miss and return the
/// default — `Public` — for restricted data.
///
/// So a longer policy key still matches on its leaf, at the weakest specificity,
/// and ties there resolve to the most restrictive class. An under-specified
/// question gets the conservative answer.
#[test]
fn a_leaf_only_query_falls_back_to_the_most_restrictive_match() {
    let policy = policy_from(&json!({
        "properties": {
            "usageHistory": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "recordedAt": { "type": "string", "x-disclosure": "individual" }
                }
            },
            "publicBlock": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "recordedAt": { "type": "string", "x-disclosure": "public" }
                }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_field("recordedAt"),
        Disclosure::Individual,
        "a leaf-only question must not answer Public just because one position is"
    );

    // The precise question still gets the precise answer.
    assert_eq!(
        policy.disclosure_for_path(
            &["publicBlock", "recordedAt"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Public
    );
}

/// A product group's classes still stop at its own payload. Path matching is a
/// change to how a key is found within a scope, not to where a scope reaches.
#[test]
fn paths_do_not_let_a_product_group_reclassify_the_envelope() {
    let policy = policy_from(&json!({
        "properties": {
            "manufacturer": {
                "type": "object",
                "x-disclosure": "public",
                "properties": {
                    "address": { "type": "string", "x-disclosure": "restricted" }
                }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(&["manufacturer", "address"], DocumentScope::Envelope),
        Disclosure::Public,
        "a schema class must not reach an envelope field, however precisely it is written"
    );
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn policy_from(schema: &serde_json::Value) -> ProductGroupAccessPolicy {
    ProductGroupAccessPolicy::from_schema("test", "1.0.0", &schema.to_string())
        .expect("schema has a root properties map")
}
