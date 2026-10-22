//! Every disclosure class declared in a shipped schema resolves to itself.
//!
//! Separate from `path_matching_tests`, which exercises the matcher on synthetic
//! policies: this one asks the only question that matters about real data —
//! did any actual field's class move?

use serde_json::Value;

use crate::{Audience, Disclosure};

use super::policy::{DocumentScope, ProductGroupAccessPolicy};

/// Whether the class `answered` is withheld from every audience that `declared`
/// withholds from, and perhaps more.
fn at_least_as_restrictive(answered: Disclosure, declared: Disclosure) -> bool {
    [
        Audience::Public,
        Audience::LegitimateInterest,
        Audience::Authority,
    ]
    .iter()
    .all(|a| !a.may_see(answered) || a.may_see(declared))
}

/// **Every declared class in every shipped schema resolves to itself.**
///
/// The safety property for this change. Recording classes by path rather than by
/// leaf rewrites how every schema is read, so the question is not whether the
/// new matcher is expressive — it is whether any real field's class moved. This
/// walks every version of every product group, and for each declared
/// `x-disclosure` asserts the policy answers that class at that path.
///
/// A field whose class silently changed here would be either a leak or an
/// over-redaction, depending on direction, and neither is visible by reading a
/// diff of the matcher.
///
/// # Two kinds of visit
///
/// A **position** is a property reached through the schema's own `properties`,
/// `items` and `$ref` chain, so the policy holds a key for its full path and the
/// answer must be exactly the declared class.
///
/// A **floor** visit is a definition read at the empty path, under its bare leaf
/// name. The policy keeps that key as a fail-closed fallback for a reference the
/// walk could not follow, and two definitions that share a member name share the
/// key, which resolves to the more restrictive of their classes. So a floor key
/// may be stricter than a declaration and may never be looser. Holding it to an
/// exact answer made a definition's members impossible to relabel whenever
/// another definition in the same schema declared the same name more openly,
/// though no document position is ever answered by that key
/// (`no_shipped_schema_refers_to_a_definition_from_a_construct_the_collector_skips`).
#[test]
fn every_declared_property_resolves_to_its_own_class() {
    let registry = crate::schemas::VersionedSchemaRegistry::new();
    let mut checked = 0usize;
    let mut wrong: Vec<String> = Vec::new();

    for product_group in registry.product_groups() {
        for version in registry.versions_for(product_group) {
            let json_text = registry
                .get(product_group, version)
                .expect("the registry listed it");
            let schema: serde_json::Value =
                serde_json::from_str(json_text).expect("schema is valid JSON");
            let policy = ProductGroupAccessPolicy::from_schema(
                product_group,
                &version.to_string(),
                json_text,
            )
            .expect("policy builds from a listed schema");

            let mut active_refs: Vec<String> = Vec::new();
            declared_paths(
                &schema,
                &[],
                &schema,
                &mut active_refs,
                false,
                &mut |path: &[String], class, floor: bool| {
                    let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();
                    let answered =
                        policy.disclosure_for_path(&borrowed, DocumentScope::ProductGroupData);
                    checked += 1;
                    let holds = if floor {
                        at_least_as_restrictive(answered, class)
                    } else {
                        answered == class
                    };
                    if !holds {
                        wrong.push(format!(
                        "{product_group} v{version}: {} declares {class:?} but resolves to {answered:?}{}",
                        borrowed.join("."),
                        if floor { " (a floor key may be stricter than its declaration, never looser)" } else { "" }
                    ));
                    }
                },
            );
        }
    }

    assert!(checked > 0, "no declared properties were checked at all");
    assert!(
        wrong.is_empty(),
        "{} of {checked} declared classes do not resolve to themselves: {wrong:#?}",
        wrong.len()
    );
}

/// **The floor answers for no shipped position.**
///
/// A definition's bare-leaf key exists for a reference the policy walk cannot
/// follow: one under `patternProperties`, `if` / `then` / `else`, `not`,
/// `dependentSchemas` and the like, a tuple-form `items`, or a `$ref` that is not
/// a local pointer that resolves. Everywhere else the walk records the definition
/// under the path of whatever referred to it, and the longer key wins.
///
/// So the floor's one inexactness, merging two definitions that share a member
/// name to the stricter class, can only answer for a position this test would
/// find. It finds none in any shipped schema, which is what lets
/// `every_declared_property_resolves_to_its_own_class` hold a floor key to
/// "never looser" instead of "exact". The day a schema adds such a reference
/// this fails, and whoever adds it decides whether the floor must become exact
/// before it answers for a real field.
#[test]
fn no_shipped_schema_refers_to_a_definition_from_a_construct_the_collector_skips() {
    let registry = crate::schemas::VersionedSchemaRegistry::new();
    let mut checked = 0usize;
    let mut skipped: Vec<String> = Vec::new();

    for product_group in registry.product_groups() {
        for version in registry.versions_for(product_group) {
            let json_text = registry
                .get(product_group, version)
                .expect("the registry listed it");
            let schema: Value = serde_json::from_str(json_text).expect("schema is valid JSON");
            checked += 1;
            for site in refs_the_collector_skips(&schema) {
                skipped.push(format!("{product_group} v{version}: {site}"));
            }
        }
    }

    assert!(checked > 0, "no schemas were scanned at all");
    assert!(
        skipped.is_empty(),
        "a $ref the policy walk does not follow, so its definition is answered only \
         by the bare-leaf floor: {skipped:#?}"
    );
}

/// The scan is only worth having if it catches each shape it is for, and passes
/// a property that merely shares a keyword's name.
#[test]
fn the_ref_scan_catches_what_it_is_for() {
    let schema = serde_json::json!({
        "properties": {
            "followed": { "items": { "$ref": "#/definitions/d" } },
            "propertyNamedNot": {
                "properties": { "not": { "$ref": "#/definitions/d" } }
            },
            "underPattern": {
                "patternProperties": { "^x-": { "$ref": "#/definitions/d" } }
            },
            "underThen": { "if": {}, "then": { "$ref": "#/definitions/d" } },
            "tuple": { "items": [{ "$ref": "#/definitions/d" }] },
            "remote": { "$ref": "other.json#/definitions/d" },
            "dangling": { "$ref": "#/definitions/missing" }
        },
        "definitions": { "d": { "type": "object" } }
    });

    let mut found = refs_the_collector_skips(&schema);
    found.sort();
    assert_eq!(
        found,
        [
            "/properties/dangling",
            "/properties/remote",
            "/properties/tuple/items/0",
            "/properties/underPattern/patternProperties/^x-",
            "/properties/underThen/then",
        ]
    );
}

/// Every node carrying a `$ref` the policy walk will not follow, as a JSON
/// pointer to that node: one under a construct `collect_disclosures` does not
/// descend, or one that is not a local pointer that resolves.
fn refs_the_collector_skips(root: &Value) -> Vec<String> {
    let mut out = Vec::new();
    scan_for_skipped_refs(root, "", false, root, &mut out);
    out
}

fn scan_for_skipped_refs(
    node: &Value,
    pointer: &str,
    unwalked: bool,
    root: &Value,
    out: &mut Vec<String>,
) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(target) = object.get("$ref").and_then(Value::as_str) {
        let followed = target
            .strip_prefix('#')
            .and_then(|p| root.pointer(p))
            .is_some();
        if unwalked || !followed {
            out.push(if pointer.is_empty() {
                "/".to_owned()
            } else {
                pointer.to_owned()
            });
        }
    }

    for (keyword, value) in object {
        let here = format!("{pointer}/{keyword}");
        match keyword.as_str() {
            // Maps keyed by a name the schema chose, so a key is never a keyword.
            "properties" | "definitions" | "$defs" => {
                scan_members(value, &here, unwalked, root, out);
            }
            "patternProperties" | "dependentSchemas" | "dependencies" => {
                scan_members(value, &here, true, root, out);
            }
            "items" => match value.as_array() {
                Some(tuple) => scan_elements(tuple, &here, true, root, out),
                None => scan_for_skipped_refs(value, &here, unwalked, root, out),
            },
            "additionalProperties" => scan_for_skipped_refs(value, &here, unwalked, root, out),
            "allOf" | "anyOf" | "oneOf" => {
                if let Some(branches) = value.as_array() {
                    scan_elements(branches, &here, unwalked, root, out);
                }
            }
            "prefixItems" => {
                if let Some(elements) = value.as_array() {
                    scan_elements(elements, &here, true, root, out);
                }
            }
            "if"
            | "then"
            | "else"
            | "not"
            | "contains"
            | "propertyNames"
            | "additionalItems"
            | "unevaluatedProperties"
            | "unevaluatedItems" => {
                scan_for_skipped_refs(value, &here, true, root, out);
            }
            _ => {}
        }
    }
}

fn scan_members(
    members: &Value,
    pointer: &str,
    unwalked: bool,
    root: &Value,
    out: &mut Vec<String>,
) {
    if let Some(map) = members.as_object() {
        for (name, schema) in map {
            scan_for_skipped_refs(schema, &format!("{pointer}/{name}"), unwalked, root, out);
        }
    }
}

fn scan_elements(
    elements: &[Value],
    pointer: &str,
    unwalked: bool,
    root: &Value,
    out: &mut Vec<String>,
) {
    for (index, schema) in elements.iter().enumerate() {
        scan_for_skipped_refs(schema, &format!("{pointer}/{index}"), unwalked, root, out);
    }
}

/// Visit every declared `(path, class, floor)` triple in a schema, mirroring how
/// `collect_disclosures` walks it: only `properties` adds a segment,
/// `definitions` / `$defs` are walked at the root for the bare-leaf floor, and
/// local `$ref` pointers are followed so a definition is also visited under the
/// path of whatever referred to it.
///
/// `floor` is true once the walk is inside a `definitions` / `$defs` block, where
/// every key it reports is a bare-leaf fallback rather than a position.
///
/// **This is a mirror, and its limits follow from that.** It reproduces the
/// collector's rules rather than deriving them independently, so a rule that is
/// wrong in both places satisfies it. What it does catch is the collector and
/// the matcher disagreeing — including a position whose key was merged with
/// another declaration of a different class, which fails the exact assertion.
/// For a check that does not share the collector's assumptions, see
/// `ref_path_tests::the_filter_redacts_by_path_through_a_ref`, which redacts a
/// document and inspects what survives.
fn declared_paths(
    node: &Value,
    path: &[String],
    root: &Value,
    active_refs: &mut Vec<String>,
    floor: bool,
    visit: &mut impl FnMut(&[String], Disclosure, bool),
) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(pointer) = object.get("$ref").and_then(serde_json::Value::as_str)
        && let Some(target) = pointer.strip_prefix('#').and_then(|p| root.pointer(p))
        && !active_refs.iter().any(|seen| seen == pointer)
    {
        active_refs.push(pointer.to_owned());
        declared_paths(target, path, root, active_refs, floor, visit);
        active_refs.pop();
    }

    if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            let mut child = path.to_vec();
            child.push(name.clone());
            if let Some(class) = prop
                .get("x-disclosure")
                .and_then(serde_json::Value::as_str)
                .and_then(parse_class)
            {
                visit(&child, class, floor);
            }
            declared_paths(prop, &child, root, active_refs, floor, visit);
        }
    }

    if let Some(node) = object.get("items") {
        declared_paths(node, path, root, active_refs, floor, visit);
    }
    if let Some(node) = object.get("additionalProperties") {
        declared_paths(node, &[], root, active_refs, floor, visit);
    }
    for key in ["definitions", "$defs"] {
        if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
            for definition in block.values() {
                declared_paths(definition, &[], root, active_refs, true, visit);
            }
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
            for branch in branches {
                declared_paths(branch, path, root, active_refs, floor, visit);
            }
        }
    }
}

fn parse_class(token: &str) -> Option<Disclosure> {
    match token {
        "public" => Some(Disclosure::Public),
        "restricted" => Some(Disclosure::Restricted),
        "conformity" => Some(Disclosure::Conformity),
        "individual" => Some(Disclosure::Individual),
        _ => None,
    }
}
