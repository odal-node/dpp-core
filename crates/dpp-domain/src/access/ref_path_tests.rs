//! A `$ref`'d definition carries its classes to every path that refers to it.
//!
//! `path_matching_tests` exercises the matcher on inline nesting. This file
//! covers the case the shipped schemas actually use: every shared leaf in this
//! crate's schemas lives in a `definitions` block reached by `$ref`, so a
//! definition that could not be positioned was a definition whose class could
//! not be stated precisely — whatever the matcher could express.
//!
//! # Which of these bite
//!
//! Verified by disabling `$ref` resolution and re-running. Five of the six here
//! fail: `two_definitions_declaring_one_leaf_are_classified_apart`,
//! `a_named_path_still_beats_the_floor`,
//! `a_definition_lands_on_every_path_that_refers_to_it`,
//! `the_battery_material_definition_is_positioned` and
//! `the_filter_redacts_by_path_through_a_ref`.
//!
//! `an_unreached_position_still_gets_the_definition_class` passes either way, by
//! design — it pins the floor, which this change must leave alone. The walk's
//! other invariants live in `ref_walk_tests`.
//!
//! Worth knowing when reading them: asserting a *resolved class* often cannot
//! separate the two implementations, because the bare-leaf floor answers the
//! same question correctly whenever nothing competes with it. Where that is the
//! case the test asserts on the recorded keys, or redacts a document and looks
//! at what survives.

use serde_json::json;

use crate::{Audience, Disclosure};

use super::filter::filter_by_audience_in_scope;
use super::policy::{DocumentScope, ProductGroupAccessPolicy};

/// **The case #144 was opened about, and the one #200 could not reach.**
///
/// Two definitions declaring the same leaf now yield distinct keys, so one may
/// be `Restricted` while its twin stays `Public`. Keyed by leaf alone the
/// collision merged to the restrictive class and redacted both — over-redacting
/// Annex III content the public passport is required to carry.
#[test]
fn two_definitions_declaring_one_leaf_are_classified_apart() {
    let policy = policy_from(&json!({
        "properties": {
            "anodeMaterial": {
                "type": "array",
                "items": { "$ref": "#/definitions/materialComposition" }
            },
            "criticalRawMaterials": {
                "type": "array",
                "items": { "$ref": "#/definitions/criticalRawMaterial" }
            }
        },
        "definitions": {
            "materialComposition": {
                "properties": { "name": { "x-disclosure": "restricted" } }
            },
            "criticalRawMaterial": {
                "properties": { "name": { "x-disclosure": "public" } }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(&["anodeMaterial", "name"], DocumentScope::ProductGroupData),
        Disclosure::Restricted,
        "the restricted definition governs the path that refers to it"
    );
    assert_eq!(
        policy.disclosure_for_path(
            &["criticalRawMaterials", "name"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Public,
        "and its twin is not dragged along with it"
    );
}

/// One definition, several referrers: the class is keyed under each of their
/// paths.
///
/// Asserted on the recorded **keys** rather than on a resolved class, because a
/// resolved class cannot tell the two implementations apart here — with a single
/// definition and no competing declaration, the bare-leaf floor answers
/// `["anodeMaterial", "weightPct"]` correctly whether or not the `$ref` was
/// followed. The keys are what changed.
#[test]
fn a_definition_lands_on_every_path_that_refers_to_it() {
    let policy = policy_from(&json!({
        "properties": {
            "anodeMaterial": { "items": { "$ref": "#/definitions/material" } },
            "cathodeMaterial": { "items": { "$ref": "#/definitions/material" } },
            "electrolyteMaterial": { "items": { "$ref": "#/definitions/material" } }
        },
        "definitions": {
            "material": {
                "properties": { "weightPct": { "x-disclosure": "restricted" } }
            }
        }
    }));

    for parent in ["anodeMaterial", "cathodeMaterial", "electrolyteMaterial"] {
        assert!(
            policy
                .field_disclosure
                .contains_key(&format!("{parent}.weightPct")),
            "{parent} refers to the definition, so its class must be keyed under \
             that path and not only as a bare leaf: {:?}",
            policy.field_disclosure.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            policy.disclosure_for_path(&[parent, "weightPct"], DocumentScope::ProductGroupData),
            Disclosure::Restricted,
            "{parent} refers to the definition and must carry its class"
        );
    }
    assert!(
        policy.field_disclosure.contains_key("weightPct"),
        "and the floor is kept alongside them"
    );
}

/// **The floor.** A definition keeps its bare-leaf key as well as its referring
/// paths, so a position this walk never reached still gets the class rather than
/// falling to the `Public` default.
///
/// This walk descends `properties`, `items`, `additionalProperties`, the three
/// combinators and `$ref` — not `patternProperties`, `if` / `then` / `else`,
/// `not` or `dependentSchemas`. A definition referred to from one of those has
/// no referring path recorded, and without the leaf its class would disappear
/// silently. Over-applying is the survivable direction; under-applying is a leak.
#[test]
fn an_unreached_position_still_gets_the_definition_class() {
    let policy = policy_from(&json!({
        "properties": {
            "anodeMaterial": { "items": { "$ref": "#/definitions/material" } },
            "elsewhere": {
                "patternProperties": {
                    "^x-": { "$ref": "#/definitions/material" }
                }
            }
        },
        "definitions": {
            "material": {
                "properties": { "weightPct": { "x-disclosure": "restricted" } }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(
            &["elsewhere", "xCustom", "weightPct"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Restricted,
        "a reference through a construct the walk does not descend must not \
         lose the class — the bare leaf is what covers it"
    );
}

/// The floor never overrides a position that was named, because a referring path
/// always scores higher than the one-segment leaf.
#[test]
fn a_named_path_still_beats_the_floor() {
    let policy = policy_from(&json!({
        "properties": {
            "restrictedHolder": { "items": { "$ref": "#/definitions/secret" } },
            "publicHolder": { "items": { "$ref": "#/definitions/open" } }
        },
        "definitions": {
            "secret": { "properties": { "value": { "x-disclosure": "restricted" } } },
            "open": { "properties": { "value": { "x-disclosure": "public" } } }
        }
    }));

    // The floor merges both declarations of `value` to the restrictive class.
    assert_eq!(
        policy.disclosure_for_path(&["value"], DocumentScope::ProductGroupData),
        Disclosure::Restricted,
        "an under-specified query gets the conservative answer"
    );
    // A named path is a more specific statement and wins over it.
    assert_eq!(
        policy.disclosure_for_path(&["publicHolder", "value"], DocumentScope::ProductGroupData),
        Disclosure::Public
    );
}

/// The shipped battery schema, which is where this actually mattered.
///
/// `materialComposition.weightPct` is `Restricted` and the definition is `$ref`'d
/// from the three electrode arrays. Before `$ref` resolution the class existed
/// only as bare `weightPct`; it is now positioned as well, which is what lets a
/// future schema relax or tighten one electrode without touching the others.
#[test]
fn the_battery_material_definition_is_positioned() {
    let policy = ProductGroupAccessPolicy::for_schema_version("battery", "2.6.0")
        .expect("battery 2.6.0 is registered");

    for parent in ["anodeMaterial", "cathodeMaterial", "electrolyteMaterial"] {
        assert!(
            policy
                .field_disclosure
                .contains_key(&format!("{parent}.weightPct")),
            "{parent}.weightPct should be recorded as its own key, not only as \
             bare weightPct: {:?}",
            policy.field_disclosure.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            policy.disclosure_for_path(&[parent, "weightPct"], DocumentScope::ProductGroupData),
            Disclosure::Restricted
        );
    }

    // And the leaf that shares its name across two definitions resolves per
    // position rather than as one merged class.
    assert_eq!(
        policy.disclosure_for_path(
            &["criticalRawMaterials", "casNumber"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Public,
        "Annex XIII point 1(b) puts critical raw materials on the public tier"
    );
}

/// **The filter agrees with the collector**, checked by redacting a document
/// rather than by querying the policy.
///
/// Every other test here — and `every_declared_property_resolves_to_its_own_class`
/// — asks the policy a question phrased in the collector's own terms, so a walk
/// that recorded a definition under a consistently wrong path would satisfy all
/// of them. This one runs a document through `filter_by_audience_in_scope` and
/// looks at what survives, which is what a reader actually receives. The path the
/// filter builds and the key the collector recorded have to be the same string,
/// or the field is not redacted.
///
/// The document is shaped like the battery payload — two arrays of objects
/// sharing leaf names, one reached through a restricted definition — because
/// that is the shape the schemas use. It is synthetic because the shipped
/// schemas have no live collision: battery declares `name` and `casNumber`
/// `Public` in both definitions, and `anodeMaterial` carries a `Restricted`
/// class on the array itself, so the whole subtree leaves at `Public` before any
/// per-leaf class is reached. A real payload cannot demonstrate a capability no
/// shipped schema exercises yet.
#[test]
fn the_filter_redacts_by_path_through_a_ref() {
    let policy = policy_from(&json!({
        "properties": {
            "anodeMaterial": {
                "type": "array",
                "items": { "$ref": "#/definitions/materialComposition" }
            },
            "criticalRawMaterials": {
                "type": "array",
                "items": { "$ref": "#/definitions/criticalRawMaterial" }
            }
        },
        "definitions": {
            "materialComposition": {
                "properties": {
                    "name": { "x-disclosure": "public" },
                    "weightPct": { "x-disclosure": "restricted" },
                    "casNumber": { "x-disclosure": "restricted" }
                }
            },
            "criticalRawMaterial": {
                "properties": {
                    "name": { "x-disclosure": "public" },
                    "casNumber": { "x-disclosure": "public" }
                }
            }
        }
    }));

    let document = json!({
        "anodeMaterial": [{ "name": "graphite", "weightPct": 42.5, "casNumber": "7782-42-5" }],
        "criticalRawMaterials": [{ "name": "cobalt", "casNumber": "7440-48-4" }]
    });

    let public = filter_by_audience_in_scope(
        &document,
        &policy,
        Audience::Public,
        DocumentScope::ProductGroupData,
    )
    .filtered_data;

    let anode = &public["anodeMaterial"][0];
    assert!(
        anode.get("weightPct").is_none() && anode.get("casNumber").is_none(),
        "the restricted definition's fields must not reach an anonymous \
         reader: {anode}"
    );
    assert!(
        anode.get("name").is_some(),
        "its Public sibling in the same definition survives: {anode}"
    );

    let crm = &public["criticalRawMaterials"][0];
    assert!(
        crm.get("casNumber").is_some(),
        "and the same leaf name under the public definition is not dragged under \
         with it — the whole point, and unexpressible before the ref was \
         followed: {crm}"
    );
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn policy_from(schema: &serde_json::Value) -> ProductGroupAccessPolicy {
    ProductGroupAccessPolicy::from_schema("test", "1.0.0", &schema.to_string())
        .expect("schema has a root properties map")
}
