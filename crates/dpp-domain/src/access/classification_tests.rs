//! Every schema property declares a disclosure class, and no field name is
//! declared in two classes across the schemas that share a scope.

use crate::Disclosure;

use super::policy::ProductGroupAccessPolicy;

/// Every property of the current battery schema declares an `x-disclosure`
/// class — **at every depth**.
///
/// This is the guarantee the annotation exists to provide. The catalog map it
/// replaces has no equivalent: a field added there without an entry silently
/// defaults to `Public`, and for Annex XIII point 2, 3 or 4 content that is a
/// leak rather than an omission. Co-locating the class with the property makes
/// the gap visible in the same diff; this test makes it fail the build.
///
/// **A misspelt class is checked too, not just a missing one.**
/// `ProductGroupAccessPolicy::from_schema` matches the four known tokens and drops
/// anything else, so `"restrcted"` produces no map entry and the field falls to
/// `default_disclosure` — `Public`. A typo in one character therefore publishes
/// a restricted field, and asserting only that *some* string is present would
/// not catch it.
///
/// **Depth is the part this used to miss.** It walked `schema["properties"]`
/// and stopped, so 184 properties nested inside objects, array `items` and
/// `definitions` blocks were unchecked — and the constructor could not have
/// read them anyway. Both halves are fixed together: reading one without
/// gating the other leaves a field that silently defaults to public, and
/// gating one without reading the other rejects schemas the code then ignores.
#[test]
fn every_property_declares_a_valid_disclosure_class() {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let (version, json) = reg.latest("battery").expect("battery schema exists");
    let schema: serde_json::Value = serde_json::from_str(json).expect("valid JSON");

    let bad = undeclared_properties(&schema);
    assert!(
        bad.is_empty(),
        "battery v{version}: these properties declare no usable x-disclosure class, so they \
         would default to public: {bad:?}"
    );
}

/// The four tokens `ProductGroupAccessPolicy::from_schema` recognises. Anything else
/// is dropped by that constructor and falls through to the public default.
const VALID_DISCLOSURE_TOKENS: [&str; 4] = ["public", "restricted", "conformity", "individual"];

/// Every property in `schema`, at any depth, that declares no usable class.
///
/// Mirrors the traversal `ProductGroupAccessPolicy::from_schema` performs, for the
/// same reason the two are described together: a gate that walks a different
/// tree from the constructor is a gate over a different schema.
fn undeclared_properties(schema: &serde_json::Value) -> Vec<(String, String)> {
    fn walk(node: &serde_json::Value, path: &str, out: &mut Vec<(String, String)>) {
        let Some(object) = node.as_object() else {
            return;
        };
        if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in properties {
                let child = if path.is_empty() {
                    name.clone()
                } else {
                    format!("{path}.{name}")
                };
                let class = prop
                    .get("x-disclosure")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("<missing>");
                if !VALID_DISCLOSURE_TOKENS.contains(&class) {
                    out.push((child.clone(), class.to_owned()));
                }
                walk(prop, &child, out);
            }
        }
        for key in ["items", "additionalProperties"] {
            if let Some(child) = object.get(key) {
                walk(child, &format!("{path}[]"), out);
            }
        }
        for key in ["definitions", "$defs"] {
            if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
                for (name, definition) in block {
                    walk(definition, &format!("{key}.{name}"), out);
                }
            }
        }
        for key in ["allOf", "anyOf", "oneOf"] {
            if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
                for branch in branches {
                    walk(branch, path, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(schema, "", &mut out);
    out
}

/// No schema declares one field name in two different classes.
///
/// Matching is by normalized **leaf name** at any depth, so `materialComposition.name`
/// and `criticalRawMaterial.name` are one key. A schema declaring them
/// differently is asking for something the matcher cannot express, and the
/// constructor would have to pick — silently, and for every field of that name
/// in the document.
///
/// It picks the more restrictive one, which is the safe direction but is still
/// a guess at what the author meant. This test means no shipped schema ever
/// makes it guess. It is also what keeps `disclosure_for_field` honest: the
/// tie-break exists for hand-built policies, not for these.
///
/// **This is the constraint that decides how a nested field may be classified.**
/// A nested property whose leaf name is shared with a more permissive field
/// elsewhere in the same schema cannot be restricted by annotation alone —
/// restricting it would restrict the twin. Where that arises today the parent
/// object carries the restriction instead, and the filter removes the whole
/// subtree before ever reaching the leaf. Expressing it on the leaf itself
/// needs a path-aware matcher, which this crate does not have.
#[test]
fn no_schema_declares_one_field_name_in_two_classes() {
    use std::collections::HashMap;

    let reg = crate::schemas::VersionedSchemaRegistry::new();
    for product_group in reg.product_groups() {
        for version in reg.versions_for(product_group) {
            let json = reg.get(product_group, version).expect("registry listed it");
            let schema: serde_json::Value = serde_json::from_str(json).expect("valid JSON");

            let mut seen: HashMap<String, (String, Disclosure)> = HashMap::new();
            let mut clashes: Vec<String> = Vec::new();
            collect_declared(&schema, "", &mut |path: &str, name: &str, class| {
                let key: String = name
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .map(|c| c.to_ascii_lowercase())
                    .collect();
                if let Some((first_path, first)) = seen.get(&key) {
                    if *first != class {
                        clashes.push(format!(
                            "'{name}' is {first:?} at {first_path} and {class:?} at {path}"
                        ));
                    }
                } else {
                    seen.insert(key, (path.to_owned(), class));
                }
            });

            assert!(
                clashes.is_empty(),
                "{product_group} v{version}: one field name declared in two classes, so the policy \
                 cannot express both: {clashes:?}"
            );
        }
    }
}

/// Visit every declared `(path, name, class)` triple in a schema.
fn collect_declared(
    node: &serde_json::Value,
    path: &str,
    visit: &mut impl FnMut(&str, &str, Disclosure),
) {
    let Some(object) = node.as_object() else {
        return;
    };
    if let Some(properties) = object.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            let child = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if let Some(class) = prop
                .get("x-disclosure")
                .and_then(serde_json::Value::as_str)
                .and_then(|t| match t {
                    "public" => Some(Disclosure::Public),
                    "restricted" => Some(Disclosure::Restricted),
                    "conformity" => Some(Disclosure::Conformity),
                    "individual" => Some(Disclosure::Individual),
                    _ => None,
                })
            {
                visit(&child, name, class);
            }
            collect_declared(prop, &child, visit);
        }
    }
    for key in ["items", "additionalProperties"] {
        if let Some(child) = object.get(key) {
            collect_declared(child, &format!("{path}[]"), visit);
        }
    }
    for key in ["definitions", "$defs"] {
        if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
            for (name, definition) in block {
                collect_declared(definition, &format!("{key}.{name}"), visit);
            }
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
            for branch in branches {
                collect_declared(branch, path, visit);
            }
        }
    }
}

/// Every schema version of every product group yields a policy, and every property in
/// each declares a class.
///
/// The backfill guarantee. `for_schema_version` fails closed, so a schema
/// version left un-annotated would not misbehave — it would refuse to serve
/// that passport at all. Older versions carry today's classes because no
/// passport has ever been published under any of them; there is no historical
/// map to preserve, and this is the last moment that is true.
#[test]
fn every_product_group_version_yields_a_fully_classified_policy() {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let mut checked = 0usize;
    for product_group in reg.product_groups() {
        for version in reg.versions_for(product_group) {
            let json = reg.get(product_group, version).expect("registry listed it");
            let schema: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
            let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) else {
                continue;
            };
            // Missing *and* misspelt: both land the field on the public
            // default, so both have to fail the build. Walked at every depth —
            // a nested property with no class defaults to public exactly as a
            // top-level one does.
            let _ = properties;
            let undeclared = undeclared_properties(&schema);
            assert!(
                undeclared.is_empty(),
                "{product_group} v{version}: unclassified properties {undeclared:?}"
            );
            assert!(
                ProductGroupAccessPolicy::for_schema_version(product_group, &version.to_string())
                    .is_some(),
                "{product_group} v{version} yields no policy"
            );
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} schema versions walked");
}
