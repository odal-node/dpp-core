//! The reader that turns product-group schemas into JSON-LD terms, held to
//! schemas written for the purpose.
//!
//! The embedded schemas are checked against the finished context in
//! `jsonld_product_group_terms.rs`. These pin the reader's own rules, including
//! the ones that matter only when a schema does something none of today's does.

use serde_json::{Value, json};

use super::product_terms::scope_from_schemas;

fn scope_of(schemas: Vec<Value>) -> Value {
    scope_from_schemas(
        schemas
            .into_iter()
            .enumerate()
            .map(|(index, schema)| (format!("schema {index}"), schema)),
    )
}

#[test]
fn a_field_gets_an_iri_that_is_its_path_under_the_group_data() {
    let scope = scope_of(vec![json!({
        "properties": {
            "chemistry": { "type": "string" },
            "anode": { "type": "object", "properties": { "name": { "type": "string" } } }
        }
    })]);

    assert_eq!(scope["chemistry"], json!("dpp:productGroupData.chemistry"));
    assert_eq!(
        scope["anode"]["@id"],
        json!("dpp:productGroupData.anode"),
        "a field with members is defined by an object that carries its own scope"
    );
    assert_eq!(
        scope["anode"]["@context"]["name"],
        json!("dpp:productGroupData.anode.name")
    );
}

#[test]
fn every_scope_stops_propagating() {
    let scope = scope_of(vec![json!({
        "properties": { "a": { "properties": { "b": { "properties": { "c": {} } } } } }
    })]);

    for node in [
        &scope,
        &scope["a"]["@context"],
        &scope["a"]["@context"]["b"]["@context"],
    ] {
        assert_eq!(node["@propagate"], json!(false));
    }
}

#[test]
fn schemas_are_unioned_so_an_older_version_keeps_its_terms() {
    let scope = scope_of(vec![
        json!({ "properties": { "gtin": { "type": "string" }, "shared": { "type": "string" } } }),
        json!({ "properties": { "shared": { "type": "string" }, "added": { "type": "string" } } }),
    ]);

    for key in ["gtin", "shared", "added"] {
        assert!(scope.get(key).is_some(), "`{key}` lost its term");
    }
}

#[test]
fn the_members_of_two_versions_of_one_object_are_merged() {
    let scope = scope_of(vec![
        json!({ "properties": { "cell": { "properties": { "mass": {} } } } }),
        json!({ "properties": { "cell": { "properties": { "volume": {} } } } }),
    ]);

    let cell = &scope["cell"]["@context"];
    assert!(cell.get("mass").is_some() && cell.get("volume").is_some());
}

#[test]
fn references_items_and_every_combinator_describe_the_same_position() {
    let scope = scope_of(vec![json!({
        "definitions": { "material": { "properties": { "name": {}, "share": {} } } },
        "properties": {
            "parts": { "type": "array", "items": { "$ref": "#/definitions/material" } },
            "either": { "oneOf": [
                { "properties": { "left": {} } },
                { "properties": { "right": {} } }
            ] },
            "both": { "allOf": [ { "properties": { "x": {} } }, { "properties": { "y": {} } } ] },
            "any": { "anyOf": [ { "properties": { "p": {} } } ] }
        }
    })]);

    // The array's element and the reference add no segment: `name` is at
    // `parts.name`, not `parts.items.name` or `parts.material.name`.
    assert_eq!(
        scope["parts"]["@context"]["name"],
        json!("dpp:productGroupData.parts.name")
    );
    for (field, members) in [("either", ["left", "right"]), ("both", ["x", "y"])] {
        for member in members {
            assert!(
                scope[field]["@context"].get(member).is_some(),
                "`{field}.{member}` has no term"
            );
        }
    }
    assert!(scope["any"]["@context"].get("p").is_some());
}

#[test]
fn a_definition_nothing_refers_to_adds_no_term() {
    let scope = scope_of(vec![json!({
        "definitions": { "orphan": { "properties": { "never": {} } } },
        "properties": { "kept": {} }
    })]);

    assert!(scope.get("never").is_none());
    assert!(scope.get("orphan").is_none());
}

#[test]
fn the_product_identifier_is_left_to_its_hand_written_term() {
    let scope = scope_of(vec![json!({
        "properties": {
            "productIdentifier": { "properties": { "gtin": {} } },
            "other": { "properties": { "productIdentifier": {} } }
        }
    })]);

    assert!(
        scope.get("productIdentifier").is_none(),
        "the generated scope must not define a second `productIdentifier`"
    );
    assert!(
        scope["other"]["@context"]
            .get("productIdentifier")
            .is_some(),
        "only the group data's own `productIdentifier` is defined elsewhere"
    );
}

#[test]
#[should_panic(expected = "uses `patternProperties`")]
fn a_construct_that_hides_fields_fails_the_build() {
    scope_of(vec![
        json!({ "patternProperties": { "^x-": { "properties": { "a": {} } } } }),
    ]);
}

#[test]
#[should_panic(expected = "uses `if`")]
fn a_conditional_that_hides_fields_fails_the_build() {
    scope_of(vec![json!({
        "properties": { "a": { "if": {}, "then": { "properties": { "b": {} } } } }
    })]);
}

#[test]
#[should_panic(expected = "keys are chosen by the document")]
fn a_map_keyed_by_the_document_fails_the_build() {
    scope_of(vec![json!({
        "properties": { "byCountry": { "additionalProperties": { "type": "number" } } }
    })]);
}

#[test]
fn a_closed_object_is_not_a_map() {
    scope_of(vec![json!({
        "properties": { "a": { "properties": { "b": {} }, "additionalProperties": false } }
    })]);
}

#[test]
#[should_panic(expected = "from inside itself")]
fn a_recursive_shape_fails_the_build() {
    scope_of(vec![json!({
        "definitions": { "node": { "properties": { "child": { "$ref": "#/definitions/node" } } } },
        "properties": { "tree": { "$ref": "#/definitions/node" } }
    })]);
}

#[test]
fn one_definition_reached_by_two_fields_is_not_recursion() {
    let scope = scope_of(vec![json!({
        "definitions": { "mass": { "properties": { "kg": {} } } },
        "properties": {
            "anode": { "$ref": "#/definitions/mass" },
            "cathode": { "$ref": "#/definitions/mass" }
        }
    })]);

    assert_eq!(
        scope["anode"]["@context"]["kg"],
        json!("dpp:productGroupData.anode.kg")
    );
    assert_eq!(
        scope["cathode"]["@context"]["kg"],
        json!("dpp:productGroupData.cathode.kg"),
        "each reference gets its own path, and so its own IRI"
    );
}

#[test]
#[should_panic(expected = "not a pointer into itself")]
fn a_reference_outside_the_schema_fails_the_build() {
    scope_of(vec![
        json!({ "properties": { "a": { "$ref": "other.json#/x" } } }),
    ]);
}

#[test]
#[should_panic(expected = "cannot be a term")]
fn a_field_named_like_a_keyword_fails_the_build() {
    scope_of(vec![json!({ "properties": { "@type": {} } })]);
}

#[test]
#[should_panic(expected = "cannot be a term")]
fn a_field_with_a_dot_in_its_name_fails_the_build() {
    scope_of(vec![json!({ "properties": { "a.b": {} } })]);
}
