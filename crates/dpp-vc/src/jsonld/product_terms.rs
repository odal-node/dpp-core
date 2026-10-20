//! The terms of `productGroupData`, generated from the product-group schemas.
//!
//! # Why this is generated
//!
//! A JSON-LD term is reachable only at the position its key occupies, and a key
//! with no term is dropped on expansion together with everything inside it. The
//! envelope's terms are a table a test compares against `Passport`. A product
//! group's fields are another matter: there are twelve groups, each with several
//! schema versions and a long list of fields, and a field is added by editing a
//! JSON file that nothing in this crate reads. A table kept by hand would be
//! missed by the next schema, and the first sign would be a consumer of the
//! linked-data form finding no battery chemistry in it.
//!
//! So the terms are read off the schemas every time the context is built. A new
//! field in an embedded schema has a term by construction, and what a schema
//! construct this reader does not follow would hide is not hidden: it fails the
//! build instead (see [`UNSUPPORTED`]).
//!
//! # The shape
//!
//! One scoped context, attached to the `productGroupData` term, that is the
//! **union** of every embedded schema version of every group. A stored passport
//! can be at any version, and a fetched one at any version this crate has ever
//! published, so the context covers all of them rather than the current one.
//!
//! The scope follows the schemas' own nesting. A field that holds an object or an
//! array of objects gets a scoped context of its own for its members, and every
//! scope sets `@propagate: false`, so a term is active at **exactly** the position
//! its key occupies and nowhere beneath it. That is the property this crate's
//! context already holds for the product identifier, and for the same reason:
//! `name` is a field of several unrelated objects, and one that propagated would
//! hand every object below it the meaning of its parent's.
//!
//! # What a term's IRI says
//!
//! `dpp:productGroupData.` and the field's dotted path from the group data, so
//! `anodeMaterial.name` is `dpp:productGroupData.anodeMaterial.name`. The path
//! has no array positions, which is how the disclosure policy writes the same
//! fields.
//!
//! **The IRI identifies a position in the group data, not a group.** Two groups
//! that both declare `countryOfOrigin` at the top level share one term, because
//! the union has to give one key at one position one meaning. That is a claim, and
//! it is the intended one: the shared fields are the cross-cutting ones (origin,
//! recycled content, substances of concern), and a consumer asking for them across
//! groups wants one IRI. Where two groups needed different meanings for one path,
//! the path would need an override here, and none does.
//!
//! Like every term in this context it names *this library's* concept and says
//! nothing about any other vocabulary: that mapping is a separate decision, and
//! `dpp-vocab` is where such a claim would be recorded.
//!
//! # What is not generated
//!
//! - `productIdentifier`. Its terms are written by hand, in the base context,
//!   because `gtin` is the one term that names GS1's vocabulary and carries a
//!   record in `dpp-vocab`. The generated scope leaves it out, so the base
//!   context's definition is the only one.
//! - `productGroup`, the tag every group's data carries on the wire. It is not a
//!   field of any schema, since the tag is the discriminator the schema is chosen
//!   by, so the envelope's term of the same name is what resolves it.

use std::collections::BTreeMap;

use dpp_domain::schemas::VersionedSchemaRegistry;
use serde_json::{Map, Value, json};

/// Fields of the group data whose terms are defined outside the generated scope.
const DEFINED_ELSEWHERE: &[&str] = &["productIdentifier"];

/// What every generated IRI starts with: our own prefix, then the position's root.
const IRI_PREFIX: &str = "dpp:productGroupData.";

/// Schema keywords whose subschemas can declare fields this reader does not
/// follow. A schema that used one would have fields with no term, which is the
/// defect this module exists to prevent, so meeting one is a build failure
/// rather than something skipped.
///
/// None is used by any embedded schema today; the list is what makes adding one
/// a decision instead of an accident.
const UNSUPPORTED: &[&str] = &[
    "patternProperties",
    "propertyNames",
    "dependencies",
    "if",
    "then",
    "else",
    "not",
    "contains",
    "additionalItems",
];

/// One field and the fields beneath it, merged over every schema that declares it.
#[derive(Default)]
struct Field {
    members: BTreeMap<String, Field>,
}

/// The context attached to `productGroupData`: one term per field of every
/// embedded product-group schema, at the position the field occupies.
///
/// # Panics
///
/// If an embedded schema is not JSON, uses a construct [`UNSUPPORTED`] lists,
/// refers to itself, or names a field that cannot be a term. All of these are
/// build-time facts about files compiled into `dpp-domain`, so they are caught by
/// the first test that builds a context rather than reached by a caller.
pub(super) fn product_group_data_scope() -> Value {
    let registry = VersionedSchemaRegistry::new();
    scope_from_schemas(registry.list().into_iter().map(|(group, version)| {
        let json = registry
            .get(group, version)
            .expect("a listed schema is held by the registry");
        let schema = serde_json::from_str(json)
            .unwrap_or_else(|error| panic!("the {group} {version} schema is not JSON: {error}"));
        (format!("the {group} {version} schema"), schema)
    }))
}

/// The scoped context for the union of `schemas`, each named for a failure.
///
/// Split from [`product_group_data_scope`] so the reader can be held to schemas
/// written for the purpose, including ones it must refuse.
pub(super) fn scope_from_schemas(schemas: impl IntoIterator<Item = (String, Value)>) -> Value {
    let mut fields = Field::default();
    for (label, schema) in schemas {
        collect(&schema, &schema, &label, &mut fields, &mut Vec::new());
    }
    Value::Object(scope(&fields, ""))
}

/// Add every field `schema` declares to `into`, following the constructs that
/// describe the same position (`$ref`, `items`, the three combinators) and
/// descending a segment for `properties` alone.
fn collect(schema: &Value, root: &Value, label: &str, into: &mut Field, active: &mut Vec<String>) {
    let Some(object) = schema.as_object() else {
        return;
    };

    for keyword in UNSUPPORTED {
        assert!(
            !object.contains_key(*keyword),
            "{label} uses `{keyword}`, which declares fields this reader does not follow, so \
             they would have no JSON-LD term"
        );
    }
    assert!(
        !object
            .get("additionalProperties")
            .is_some_and(Value::is_object),
        "{label} declares a map whose keys are chosen by the document, and a key chosen by the \
         document cannot be a term"
    );

    if let Some(pointer) = object.get("$ref").and_then(Value::as_str) {
        let target = pointer
            .strip_prefix('#')
            .and_then(|path| root.pointer(path))
            .unwrap_or_else(|| {
                panic!("{label} refers to `{pointer}`, which is not a pointer into itself")
            });
        assert!(
            !active.iter().any(|seen| seen == pointer),
            "{label} refers to `{pointer}` from inside itself, and a recursive shape has no \
             finite table of terms"
        );
        active.push(pointer.to_owned());
        collect(target, root, label, into, active);
        active.pop();
    }

    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (name, member) in properties {
            assert!(
                !name.is_empty() && !name.starts_with('@') && !name.contains(['.', ':']),
                "{label} declares a field named `{name}`, which is a keyword, a compact IRI or \
                 ambiguous in a dotted path, and cannot be a term"
            );
            collect(
                member,
                root,
                label,
                into.members.entry(name.clone()).or_default(),
                active,
            );
        }
    }

    match object.get("items") {
        Some(Value::Array(positional)) => {
            for item in positional {
                collect(item, root, label, into, active);
            }
        }
        Some(item) => collect(item, root, label, into, active),
        None => {}
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        for branch in object
            .get(keyword)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            collect(branch, root, label, into, active);
        }
    }
}

/// The scoped context for the members of `field`, found at `path`.
fn scope(field: &Field, path: &str) -> Map<String, Value> {
    // 🚨 Without this a scope would propagate to everything beneath it, and a
    // term misplaced one level up would still expand: the position would stop
    // being checked by anything.
    let mut terms = Map::new();
    terms.insert("@propagate".to_owned(), json!(false));

    for (name, member) in &field.members {
        if path.is_empty() && DEFINED_ELSEWHERE.contains(&name.as_str()) {
            continue;
        }
        let member_path = if path.is_empty() {
            name.clone()
        } else {
            format!("{path}.{name}")
        };
        let iri = format!("{IRI_PREFIX}{member_path}");
        let definition = if member.members.is_empty() {
            json!(iri)
        } else {
            json!({ "@id": iri, "@context": Value::Object(scope(member, &member_path)) })
        };
        terms.insert(name.clone(), definition);
    }
    terms
}
