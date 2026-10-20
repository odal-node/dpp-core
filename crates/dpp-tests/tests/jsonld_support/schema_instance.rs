//! A document that carries every field a product-group schema declares.
//!
//! Shared by `jsonld_product_group_terms.rs`, which holds the context to the
//! schemas, and `jsonld_oracle_corpus.rs`, which hands the same documents to an
//! independent JSON-LD processor. Each includes this file with `#[path]` rather
//! than through a `mod.rs`, so neither test binary gains a module it does not use.
//!
//! # Why this does not share code with the generator
//!
//! `dpp_vc`'s generator reads the schemas to build the terms. A check that read
//! them the same way would pass on whatever the generator misread, so this is a
//! second, separate reading, and the two must agree. It is also written the other
//! way round: it builds a *document* a processor can expand, where the generator
//! builds a table of terms.

use serde_json::{Map, Value};

/// A document with every property `schema` declares populated, at every depth.
///
/// Objects are filled member by member; an array holds one element, filled the
/// same way; a scalar gets a value of its declared type. The branches of a
/// `oneOf` are **merged**, so a union's members all appear in one object. That
/// is not a document the schema would accept, and it need not be: it is the
/// widest document an expander can be given, and a document that is valid only
/// ever carries some of the members a union declares.
pub fn full_instance(schema: &Value) -> Value {
    instance(schema, schema, &mut Vec::new())
}

fn instance(schema: &Value, root: &Value, active: &mut Vec<String>) -> Value {
    let mut members = Map::new();
    let mut element = None;
    let mut scalar = None;
    gather(
        schema,
        root,
        active,
        &mut members,
        &mut element,
        &mut scalar,
    );

    if !members.is_empty() {
        Value::Object(members)
    } else if let Some(element) = element {
        Value::Array(vec![element])
    } else {
        scalar.unwrap_or_else(|| Value::String("x".into()))
    }
}

/// Collect what `schema` says about one position: its members, the element of
/// the array it may be, or a scalar. A reference and the combinators describe the
/// same position as the schema that holds them.
fn gather(
    schema: &Value,
    root: &Value,
    active: &mut Vec<String>,
    members: &mut Map<String, Value>,
    element: &mut Option<Value>,
    scalar: &mut Option<Value>,
) {
    let Some(object) = schema.as_object() else {
        return;
    };

    if let Some(pointer) = object.get("$ref").and_then(Value::as_str) {
        assert!(
            !active.iter().any(|seen| seen == pointer),
            "`{pointer}` refers to itself, so there is no widest document"
        );
        let target = pointer
            .strip_prefix('#')
            .and_then(|path| root.pointer(path))
            .unwrap_or_else(|| panic!("`{pointer}` is not a pointer into its own schema"));
        active.push(pointer.to_owned());
        gather(target, root, active, members, element, scalar);
        active.pop();
    }

    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (name, member) in properties {
            let filled = instance(member, root, active);
            match members.get_mut(name) {
                Some(existing) => merge(existing, filled),
                None => {
                    members.insert(name.clone(), filled);
                }
            }
        }
    }

    if let Some(items) = object.get("items") {
        let filled = instance(items, root, active);
        match element {
            Some(existing) => merge(existing, filled),
            None => *element = Some(filled),
        }
    }

    for keyword in ["allOf", "anyOf", "oneOf"] {
        for branch in object
            .get(keyword)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            gather(branch, root, active, members, element, scalar);
        }
    }

    if scalar.is_none() {
        *scalar = scalar_of(object);
    }
}

/// A value of the type a leaf declares, or its `const`.
fn scalar_of(object: &Map<String, Value>) -> Option<Value> {
    if let Some(constant) = object.get("const") {
        return Some(constant.clone());
    }
    if let Some(first) = object
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
    {
        return Some(first.clone());
    }
    let declared = match object.get("type")? {
        Value::String(single) => vec![single.as_str()],
        Value::Array(several) => several.iter().filter_map(Value::as_str).collect(),
        _ => return None,
    };
    declared
        .into_iter()
        .find(|kind| *kind != "null")
        .and_then(|kind| match kind {
            "string" => Some(Value::String("x".into())),
            "number" | "integer" => Some(Value::from(1)),
            "boolean" => Some(Value::Bool(true)),
            _ => None,
        })
}

/// Fold `extra` into `into`: objects member by member, arrays by their first
/// element, and anything else left as it was.
fn merge(into: &mut Value, extra: Value) {
    match (into, extra) {
        (Value::Object(into), Value::Object(extra)) => {
            for (name, value) in extra {
                match into.get_mut(&name) {
                    Some(existing) => merge(existing, value),
                    None => {
                        into.insert(name, value);
                    }
                }
            }
        }
        (Value::Array(into), Value::Array(mut extra)) => {
            if let (Some(first), Some(extra)) = (into.first_mut(), extra.pop()) {
                merge(first, extra);
            }
        }
        (into, extra) if !into.is_object() && !into.is_array() && extra.is_object() => {
            *into = extra;
        }
        _ => {}
    }
}
