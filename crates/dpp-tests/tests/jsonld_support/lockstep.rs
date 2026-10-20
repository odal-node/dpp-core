//! Walk a document and the scoped context that should define its keys together.
//!
//! Shared by the two tests that hold the passport context to what a passport
//! carries: `jsonld_product_group_terms.rs` for `productGroupData`, and
//! `jsonld_envelope_terms.rs` for the envelope's own objects. Each includes this
//! file with `#[path]` rather than through a `mod.rs`.
//!
//! It asserts terms rather than running an expansion, because the workspace has
//! no JSON-LD processor. The PyLD oracle is what says the terms are reachable.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

/// The members of a value that is, or holds, an object.
pub fn members_of(value: &Value) -> Option<&Map<String, Value>> {
    match value {
        Value::Object(object) => Some(object),
        Value::Array(items) => items.iter().find_map(members_of),
        _ => None,
    }
}

/// Whether a term keeps its value whole as a JSON literal, so the keys inside it
/// are data and not terms.
fn is_literal(definition: &Value) -> bool {
    definition.get("@type") == Some(&Value::String("@json".into()))
}

/// Walk `document` and `scope` together, reporting each key the scope has no term
/// for at that position.
///
/// A key with members must be defined with a scoped context of its own, because
/// the scope above it does not propagate; one without need only be defined. A key
/// typed `@json` is not descended into.
pub fn missing_terms(
    document: &Map<String, Value>,
    scope: &Map<String, Value>,
    path: &str,
    out: &mut Vec<String>,
) {
    for (key, value) in document {
        let here = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        let Some(definition) = scope.get(key) else {
            out.push(here);
            continue;
        };
        if is_literal(definition) {
            continue;
        }
        if let Some(members) = members_of(value) {
            match definition.get("@context").and_then(Value::as_object) {
                Some(inner) => missing_terms(members, inner, &here, out),
                None => out.push(format!(
                    "{here} (defined, but with no scope for its members)"
                )),
            }
        }
    }
}

/// The path of every term a scope defines, as a dotted string.
pub fn scope_paths(scope: &Map<String, Value>, prefix: &str, out: &mut BTreeSet<String>) {
    for (key, definition) in scope {
        if key.starts_with('@') {
            continue;
        }
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        out.insert(path.clone());
        if let Some(inner) = definition.get("@context").and_then(Value::as_object) {
            scope_paths(inner, &path, out);
        }
    }
}

/// The path of every key a document carries, as a dotted string.
pub fn document_paths(document: &Map<String, Value>, prefix: &str, out: &mut BTreeSet<String>) {
    for (key, value) in document {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        out.insert(path.clone());
        if let Some(members) = members_of(value) {
            document_paths(members, &path, out);
        }
    }
}
