//! Every disclosure class declared in a shipped schema resolves to itself.
//!
//! Separate from `path_matching_tests`, which exercises the matcher on synthetic
//! policies: this one asks the only question that matters about real data —
//! did any actual field's class move?

use crate::Disclosure;

use super::policy::{DocumentScope, ProductGroupAccessPolicy};

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
                &mut |path: &[String], class| {
                    let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();
                    let answered =
                        policy.disclosure_for_path(&borrowed, DocumentScope::ProductGroupData);
                    checked += 1;
                    if answered != class {
                        wrong.push(format!(
                        "{product_group} v{version}: {} declares {class:?} but resolves to {answered:?}",
                        borrowed.join(".")
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

/// Visit every declared `(path, class)` pair in a schema, mirroring how
/// `collect_disclosures` walks it: only `properties` adds a segment,
/// `definitions` / `$defs` are walked at the root for the bare-leaf floor, and
/// local `$ref` pointers are followed so a definition is also visited under the
/// path of whatever referred to it.
///
/// **This is a mirror, and its limits follow from that.** It reproduces the
/// collector's rules rather than deriving them independently, so a rule that is
/// wrong in both places satisfies it. What it does catch is the collector and
/// the matcher disagreeing — including a definition declared in two blocks with
/// different classes, which resolves to the merged value and fails the
/// assertion. For a check that does not share the collector's assumptions, see
/// `ref_path_tests::the_filter_redacts_by_path_through_a_ref`, which redacts a
/// document and inspects what survives.
fn declared_paths(
    node: &serde_json::Value,
    path: &[String],
    root: &serde_json::Value,
    active_refs: &mut Vec<String>,
    visit: &mut impl FnMut(&[String], Disclosure),
) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(pointer) = object.get("$ref").and_then(serde_json::Value::as_str)
        && let Some(target) = pointer.strip_prefix('#').and_then(|p| root.pointer(p))
        && !active_refs.iter().any(|seen| seen == pointer)
    {
        active_refs.push(pointer.to_owned());
        declared_paths(target, path, root, active_refs, visit);
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
                visit(&child, class);
            }
            declared_paths(prop, &child, root, active_refs, visit);
        }
    }

    if let Some(node) = object.get("items") {
        declared_paths(node, path, root, active_refs, visit);
    }
    if let Some(node) = object.get("additionalProperties") {
        declared_paths(node, &[], root, active_refs, visit);
    }
    for key in ["definitions", "$defs"] {
        if let Some(block) = object.get(key).and_then(|b| b.as_object()) {
            for definition in block.values() {
                declared_paths(definition, &[], root, active_refs, visit);
            }
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = object.get(key).and_then(|b| b.as_array()) {
            for branch in branches {
                declared_paths(branch, path, root, active_refs, visit);
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
