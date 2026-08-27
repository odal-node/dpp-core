//! How the collector walks a schema: which constructs add a path segment, and
//! what happens at the edges of `$ref` resolution.
//!
//! Split from `ref_path_tests`, which covers what positioning a definition
//! *buys*. This file covers what following a pointer must not break — none of
//! these assert a class that moved, and none of them fail if `$ref` resolution
//! is removed. That is deliberate: the cycle tests guard against a hang, which
//! fails by never finishing rather than by asserting, and the rest pin
//! behaviour this change was required to leave alone.

use serde_json::json;

use crate::Disclosure;

use super::policy::{DocumentScope, ProductGroupAccessPolicy};

/// A definition referring to itself terminates rather than looping.
///
/// `active_refs` holds the pointers on the current descent, and a pointer
/// already there is not re-entered. Without that guard this test hangs rather
/// than fails, which is why it asserts on a value at the end.
#[test]
fn a_cyclic_ref_terminates() {
    let policy = policy_from(&json!({
        "properties": {
            "component": { "$ref": "#/definitions/component" }
        },
        "definitions": {
            "component": {
                "properties": {
                    "partNumber": { "x-disclosure": "restricted" },
                    "subComponents": {
                        "items": { "$ref": "#/definitions/component" }
                    }
                }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(
            &["component", "partNumber"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Restricted,
        "the cycle is cut, and the class before the cycle is still recorded"
    );
}

/// Mutual recursion between two definitions also terminates.
#[test]
fn a_two_step_ref_cycle_terminates() {
    let policy = policy_from(&json!({
        "properties": { "a": { "$ref": "#/definitions/a" } },
        "definitions": {
            "a": {
                "properties": {
                    "aField": { "x-disclosure": "restricted" },
                    "toB": { "$ref": "#/definitions/b" }
                }
            },
            "b": {
                "properties": {
                    "bField": { "x-disclosure": "individual" },
                    "toA": { "$ref": "#/definitions/a" }
                }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(&["a", "aField"], DocumentScope::ProductGroupData),
        Disclosure::Restricted
    );
    assert_eq!(
        policy.disclosure_for_path(&["a", "toB", "bField"], DocumentScope::ProductGroupData),
        Disclosure::Individual
    );
}

/// A `$ref` this crate cannot resolve leaves the bare-leaf key to cover it.
///
/// A pointer into another document names a file the registry does not hold.
/// Inventing a class for a shape it cannot read would be worse than letting the
/// definitions walk's leaf key apply.
#[test]
fn an_unresolvable_ref_is_not_fatal() {
    let policy = policy_from(&json!({
        "properties": {
            "external": { "$ref": "https://example.invalid/other.json#/definitions/thing" },
            "dangling": { "$ref": "#/definitions/absent" }
        },
        "definitions": {
            "present": { "properties": { "known": { "x-disclosure": "restricted" } } }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(&["known"], DocumentScope::ProductGroupData),
        Disclosure::Restricted,
        "the resolvable part of the schema is unaffected"
    );
    assert_eq!(
        policy.disclosure_for_path(&["external", "anything"], DocumentScope::ProductGroupData),
        Disclosure::Public,
        "an unreadable shape declares nothing"
    );
}

/// A map's values sit under a key the document chooses, so the subschema's
/// properties keep bare-leaf keys and match at whatever depth they land.
///
/// Carrying the map's own path would record a key no document can produce, and
/// the field would fall to the `Public` default — the fail-open direction.
#[test]
fn additional_properties_values_keep_matchable_keys() {
    let policy = policy_from(&json!({
        "properties": {
            "measurements": {
                "additionalProperties": {
                    "properties": { "rawReading": { "x-disclosure": "individual" } }
                }
            }
        }
    }));

    assert_eq!(
        policy.disclosure_for_path(
            &["measurements", "sensorA", "rawReading"],
            DocumentScope::ProductGroupData
        ),
        Disclosure::Individual,
        "the dynamic key is not knowable from the schema, so the leaf must match \
         at any depth or it matches nothing at all"
    );
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn policy_from(schema: &serde_json::Value) -> ProductGroupAccessPolicy {
    ProductGroupAccessPolicy::from_schema("test", "1.0.0", &schema.to_string())
        .expect("schema has a root properties map")
}
