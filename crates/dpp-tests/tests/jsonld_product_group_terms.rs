//! Every field of every product-group schema has a JSON-LD term, at the position
//! the field occupies.
//!
//! # What this holds the context to
//!
//! A framed passport's `productGroupData` is where a product group's own fields
//! live: the chemistry of a battery, the composition of a textile. A JSON-LD term
//! is reachable only at the position its key occupies, and a key with no term is
//! dropped on expansion together with everything inside it, with no error. The
//! terms are generated from the embedded schemas (`dpp_vc`'s `product_terms`), so
//! a new field has one by construction. This file is the check on the
//! construction: it reads the schemas a second way, with
//! `jsonld_support/schema_instance.rs`, and holds the generated context to what it
//! finds.
//!
//! It asserts terms rather than running an expansion, because the workspace has
//! no JSON-LD processor. `jsonld-oracle.yml` is the other half: it expands a
//! document carrying every field of every schema version with PyLD, which is what
//! says the terms are reachable and not merely present.

use std::collections::BTreeSet;

use dpp_domain::schemas::VersionedSchemaRegistry;
use dpp_vc::context_value;
use serde_json::{Map, Value};

#[path = "jsonld_support/lockstep.rs"]
mod lockstep;
#[path = "jsonld_support/schema_instance.rs"]
mod schema_instance;

use lockstep::{document_paths, members_of, missing_terms, scope_paths};
use schema_instance::full_instance;

/// The inline term map of the passport context.
fn base_terms() -> Map<String, Value> {
    context_value()
        .as_array()
        .expect("@context is an array")
        .iter()
        .find_map(|entry| entry.as_object())
        .expect("the context carries an inline term map")
        .clone()
}

/// The scoped context of `productGroupData`.
fn group_data_scope() -> Map<String, Value> {
    base_terms()["productGroupData"]["@context"]
        .as_object()
        .expect(
            "productGroupData defines a scoped context, or every field of every group is dropped",
        )
        .clone()
}

/// Every embedded schema, as `(label, parsed schema)`.
fn schemas() -> Vec<(String, Value)> {
    let registry = VersionedSchemaRegistry::new();
    let schemas: Vec<(String, Value)> = registry
        .list()
        .into_iter()
        .map(|(group, version)| {
            let json = registry.get(group, version).expect("a listed schema");
            (
                format!("{group} {version}"),
                serde_json::from_str(json).expect("an embedded schema is JSON"),
            )
        })
        .collect();
    assert!(
        schemas.len() > 12,
        "only {} schemas read: the registry changed shape, not the context",
        schemas.len()
    );
    schemas
}

/// 🚨 Every field of every schema version has a term at its position.
///
/// The documents are the widest each schema describes, and `productIdentifier` is
/// checked against its own hand-written scope rather than the generated one,
/// which deliberately leaves it out.
#[test]
fn every_field_of_every_schema_has_a_term_at_its_position() {
    let scope = group_data_scope();
    let identifier_scope = base_terms()["productIdentifier"]["@context"]
        .as_object()
        .expect("productIdentifier defines a scoped context")
        .clone();

    for (label, schema) in schemas() {
        let mut document = full_instance(&schema)
            .as_object()
            .expect("a product group's data is an object")
            .clone();

        if let Some(identifier) = document.remove("productIdentifier") {
            let mut missing = Vec::new();
            missing_terms(
                members_of(&identifier).expect("the product identifier has members"),
                &identifier_scope,
                "productIdentifier",
                &mut missing,
            );
            assert!(
                missing.is_empty(),
                "{label}: the product identifier carries keys its hand-written scope does not \
                 define, so a processor drops them: {missing:?}"
            );
        }

        let mut missing = Vec::new();
        missing_terms(&document, &scope, "", &mut missing);
        assert!(
            missing.is_empty(),
            "{label}: fields with no JSON-LD term at their position, which a processor drops \
             with everything inside them: {missing:?}"
        );
    }
}

/// Every term the generated scope defines is a field some schema declares.
///
/// The other direction of the check above. A term whose field does not exist is
/// inert, and it claims to define something: it is what is left behind when a
/// schema renames a field and the vocabulary does not follow.
#[test]
fn every_generated_term_is_a_field_of_some_schema() {
    let mut defined = BTreeSet::new();
    scope_paths(&group_data_scope(), "", &mut defined);

    let mut declared = BTreeSet::new();
    for (_, schema) in schemas() {
        let document = full_instance(&schema);
        document_paths(
            document.as_object().expect("group data is an object"),
            "",
            &mut declared,
        );
    }

    let inert: Vec<&String> = defined.difference(&declared).collect();
    assert!(
        inert.is_empty(),
        "terms for fields no embedded schema declares: {inert:?}"
    );
    assert!(
        defined.len() > 100,
        "only {} terms found: the scope is not what this test reads",
        defined.len()
    );
}

/// Every property name that appears in a `properties` object anywhere in any
/// schema is a term somewhere in the scope.
///
/// A blind read: it does not follow `$ref`, does not care which position a name
/// is at, and looks inside `definitions` too. It is the check that does not share
/// the other two's reading of what a schema means, so a construct both of them
/// skipped is still seen here by name.
#[test]
fn no_property_name_in_any_schema_goes_without_a_term() {
    let mut defined = BTreeSet::new();
    scope_paths(&group_data_scope(), "", &mut defined);
    let term_names: BTreeSet<&str> = defined
        .iter()
        .map(|path| path.rsplit('.').next().expect("a path has a last segment"))
        .collect();

    fn names(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(object) => {
                if let Some(properties) = object.get("properties").and_then(Value::as_object) {
                    out.extend(properties.keys().cloned());
                }
                object.values().for_each(|inner| names(inner, out));
            }
            Value::Array(items) => items.iter().for_each(|inner| names(inner, out)),
            _ => {}
        }
    }

    let mut seen = BTreeSet::new();
    for (_, schema) in schemas() {
        names(&schema, &mut seen);
    }

    // The identifier's members are defined by the hand-written scope.
    let identifier_scope = base_terms()["productIdentifier"]["@context"]
        .as_object()
        .expect("productIdentifier defines a scoped context")
        .clone();

    let unnamed: Vec<&String> = seen
        .iter()
        .filter(|name| {
            !term_names.contains(name.as_str())
                && name.as_str() != "productIdentifier"
                && !identifier_scope.contains_key(name.as_str())
        })
        .collect();
    assert!(
        unnamed.is_empty(),
        "property names a schema declares that no term is named after: {unnamed:?}"
    );
}

/// Each generated IRI is the field's own path under `dpp:productGroupData.`, and
/// each scope switches propagation off.
///
/// The IRI is the term's identity, so one that disagrees with its own key is not a
/// typo to fix later: it is a different vocabulary. And a scope that propagated
/// would make the position check above vacuous, since a term misplaced one level
/// up would still expand beneath it.
#[test]
fn every_scope_is_exact_and_every_iri_is_its_fields_path() {
    fn check(scope: &Map<String, Value>, path: &str, scopes: &mut usize, terms: &mut usize) {
        *scopes += 1;
        assert_eq!(
            scope.get("@propagate"),
            Some(&Value::Bool(false)),
            "the scope at `{path}` propagates, so a term misplaced above it would still expand"
        );
        for (key, definition) in scope.iter().filter(|(key, _)| !key.starts_with('@')) {
            *terms += 1;
            let here = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            let iri = definition
                .as_str()
                .or_else(|| definition.get("@id").and_then(Value::as_str))
                .unwrap_or_else(|| panic!("`{here}` is defined without an IRI"));
            assert_eq!(
                iri,
                format!("dpp:productGroupData.{here}"),
                "`{here}` maps to an IRI that is not its own path"
            );
            if let Some(inner) = definition.get("@context").and_then(Value::as_object) {
                check(inner, &here, scopes, terms);
            }
        }
    }

    let (mut scopes, mut terms) = (0, 0);
    check(&group_data_scope(), "", &mut scopes, &mut terms);
    assert!(
        scopes > 1 && terms > 100,
        "read {scopes} scopes and {terms} terms: the walk is not reaching the nested ones"
    );
}

/// The tag every group's data carries is not a field of any schema, and resolves
/// by the envelope's term.
///
/// `ProductGroupData` is internally tagged, so `productGroup` sits in the data on
/// the wire while the schema a passport is validated against is chosen by it and
/// never declares it. The generated scope therefore has no term for it, and the
/// scope does not propagate away the envelope's, which applies at the data's own
/// position. This pins both halves: if a schema ever declared the key, it would
/// be generated and shadow the envelope's term, and that should be a decision.
#[test]
fn the_group_tag_is_the_envelope_term_and_in_no_schema() {
    assert!(
        base_terms().contains_key("productGroup"),
        "the envelope defines no `productGroup`, so the tag in every group's data is dropped"
    );
    assert!(
        !group_data_scope().contains_key("productGroup"),
        "a schema now declares `productGroup`; its generated term shadows the envelope's"
    );
}
