//! [`SchemaDiff`] — what changed between two versions of one product group's schema.
//!
//! # Why this exists
//!
//! A schema version bump leaves no machine-readable record of what it changed.
//! The CHANGELOG entry beside it is written by hand, which makes it exactly the
//! restated claim this repository's discipline exists to prevent: the schema is
//! the fact, the prose is a copy, and copies drift.
//!
//! Three readers want the answer and none of them have it today. A reviewer
//! works out whether a bump is additive by reading raw JSON. The CHANGELOG is
//! the only record, so it is also the only thing that can be wrong. And a
//! downstream consumer repinning against a new release has no way to see which
//! product groups a schema bump touched except by diffing the files.
//!
//! # What this is not
//!
//! **Not a compatibility judgement.** `schema_compat.rs` owns the question of
//! whether a stored document still reads, and it owns it with frozen fixtures
//! rather than inference. This reports *what* changed and says nothing about
//! whether the change was allowed — two answers to one question is how they come
//! to disagree.
//!
//! It is also not a general JSON-Schema differ. It walks `properties` and
//! `required`, which is where a product-group schema states its obligations, and
//! ignores the rest of the vocabulary.
//!
//! **The known blind spot is the combinators.** A property declared with
//! `oneOf`, `anyOf` or `allOf` is reported as added, removed or retyped, but the
//! branches themselves are not walked — battery's `stateOfHealth` is the live
//! example. Adding a field inside one branch therefore shows as no change at
//! all. Walking branches means deciding which branch corresponds to which across
//! two versions, and a wrong pairing produces a confidently wrong diff, which is
//! worse than a gap this comment names.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::change::{CONSTRAINT_KEYWORDS, ChangeKind, PropertyChange};

/// Everything that changed between two versions of one product group's schema.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaDiff {
    /// Property-level changes, ordered by path.
    pub properties: Vec<PropertyChange>,
    /// Property names that became required.
    pub required_added: Vec<String>,
    /// Property names that stopped being required.
    pub required_removed: Vec<String>,
}

impl SchemaDiff {
    /// Whether the two versions declare the same properties and requirements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
            && self.required_added.is_empty()
            && self.required_removed.is_empty()
    }

    /// Whether every change only adds — no removal, no type change, no tightened
    /// constraint, no newly required property.
    ///
    /// Reported, never enforced: the additive-only rule is written down in
    /// `docs/architecture/PERSISTED-SHAPES.md` and gated by the compatibility
    /// fixtures, which test whether stored documents still read rather than
    /// inferring it from shape. This is the shape signal beside that answer, not
    /// a second answer.
    #[must_use]
    pub fn is_purely_additive(&self) -> bool {
        self.required_added.is_empty()
            && self.required_removed.is_empty()
            && self.properties.iter().all(|c| c.kind == ChangeKind::Added)
    }
}

/// Diff two schema documents of the same product group.
///
/// `prev` and `next` are whole JSON Schema documents. Walks `properties`
/// recursively — through nested objects and through an array's `items` — so a
/// field added inside a sub-object is reported at its full path rather than as a
/// change to the object containing it.
#[must_use]
pub fn diff_schemas(prev: &Value, next: &Value) -> SchemaDiff {
    let mut diff = SchemaDiff::default();
    walk(prev, next, "", &mut diff);

    let before = required_set(prev);
    let after = required_set(next);
    diff.required_added = after.difference(&before).cloned().collect();
    diff.required_removed = before.difference(&after).cloned().collect();

    diff.properties.sort_by(|a, b| a.path.cmp(&b.path));
    diff
}

fn required_set(schema: &Value) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Compare the `properties` maps of two subschemas, recursing into each shared
/// property.
fn walk(prev: &Value, next: &Value, prefix: &str, diff: &mut SchemaDiff) {
    let prev_props = properties_of(prev);
    let next_props = properties_of(next);

    for (name, next_prop) in &next_props {
        let path = join(prefix, name);
        match prev_props.get(name) {
            None => diff.properties.push(PropertyChange {
                path,
                kind: ChangeKind::Added,
                before: String::new(),
                after: type_of(next_prop),
            }),
            Some(prev_prop) => {
                compare_property(prev_prop, next_prop, &path, diff);
                // Recurse into an object's own properties, and through an
                // array's `items`, so a nested addition is not invisible.
                walk(prev_prop, next_prop, &path, diff);
                if let (Some(p), Some(n)) = (prev_prop.get("items"), next_prop.get("items")) {
                    walk(p, n, &join(&path, "[]"), diff);
                }
            }
        }
    }

    for name in prev_props.keys() {
        if !next_props.contains_key(name) {
            diff.properties.push(PropertyChange {
                path: join(prefix, name),
                kind: ChangeKind::Removed,
                before: type_of(&prev_props[name]),
                after: String::new(),
            });
        }
    }
}

/// Type and constraint comparison for one property present in both versions.
fn compare_property(prev: &Value, next: &Value, path: &str, diff: &mut SchemaDiff) {
    let (before_type, after_type) = (type_of(prev), type_of(next));
    if before_type != after_type {
        diff.properties.push(PropertyChange {
            path: path.to_owned(),
            kind: ChangeKind::TypeChanged,
            before: before_type,
            after: after_type,
        });
    }

    for keyword in CONSTRAINT_KEYWORDS {
        let before = prev.get(*keyword);
        let after = next.get(*keyword);
        if before != after {
            diff.properties.push(PropertyChange {
                path: path.to_owned(),
                kind: ChangeKind::ConstraintChanged,
                before: render_constraint(keyword, before),
                after: render_constraint(keyword, after),
            });
        }
    }
}

fn properties_of(schema: &Value) -> BTreeMap<String, Value> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

/// A property's declared type, rendered. `"—"` where the subschema declares
/// none, which a `$ref` or a bare `enum` legitimately does.
fn type_of(prop: &Value) -> String {
    match prop.get("type") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => {
            let parts: Vec<&str> = a.iter().filter_map(Value::as_str).collect();
            parts.join("|")
        }
        _ => String::new(),
    }
}

fn render_constraint(keyword: &str, value: Option<&Value>) -> String {
    match value {
        None => String::new(),
        Some(v) => format!("{keyword}={v}"),
    }
}
