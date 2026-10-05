//! A member is never visible to an audience its enclosing object is hidden from.
//!
//! The serving filter drops an object whole when the audience may not see the
//! object's own class, so today a member labelled more openly than its parent
//! leaks nothing. The label is still live policy: disclosure is recorded per
//! path, and "most specific wins" would let a member's class override its
//! parent's the moment anything classifies a member without first checking the
//! parent. A label that says a measured value of one battery is public is wrong
//! even while nothing acts on it.

use serde_json::Value;

use crate::{Audience, Disclosure};

use super::embedded::EMBEDDED;

const AUDIENCES: [Audience; 3] = [
    Audience::Public,
    Audience::LegitimateInterest,
    Audience::Authority,
];

/// A schema's `x-disclosure` token as a class. Panics on a token outside the
/// four, which the policy loader would also reject.
fn class(token: &str) -> Disclosure {
    match token {
        "public" => Disclosure::Public,
        "restricted" => Disclosure::Restricted,
        "conformity" => Disclosure::Conformity,
        "individual" => Disclosure::Individual,
        other => panic!("unknown x-disclosure token {other:?}"),
    }
}

/// Whether every audience that may see `member` may also see `parent`.
fn nests_within(member: Disclosure, parent: Disclosure) -> bool {
    AUDIENCES
        .iter()
        .all(|a| !a.may_see(member) || a.may_see(parent))
}

/// Walk `node`, reporting each member whose class an audience may see when it
/// may not see the class of the nearest enclosing object that declares one.
fn walk(
    node: &Value,
    path: &str,
    enclosing: Option<(Disclosure, &str)>,
    root: &Value,
    active_refs: &mut Vec<String>,
    out: &mut Vec<String>,
) {
    if let Some(pointer) = node.get("$ref").and_then(Value::as_str) {
        if active_refs.iter().any(|r| r == pointer) {
            return;
        }
        // A reference this walk cannot follow would hide its whole subtree from
        // the check and still pass, so it fails instead.
        let target = pointer
            .strip_prefix('#')
            .and_then(|p| root.pointer(p))
            .unwrap_or_else(|| panic!("{path}: cannot resolve $ref {pointer:?}"));
        active_refs.push(pointer.to_owned());
        walk(target, path, enclosing, root, active_refs, out);
        active_refs.pop();
    }

    let own = node.get("x-disclosure").and_then(Value::as_str).map(class);
    if let (Some(member), Some((parent, parent_path))) = (own, enclosing)
        && !nests_within(member, parent)
    {
        out.push(format!(
            "{path} is {} inside {parent_path}, which is {}",
            member.token(),
            parent.token()
        ));
    }
    let here = own.map(|c| (c, path)).or(enclosing);

    if let Some(properties) = node.get("properties").and_then(Value::as_object) {
        for (key, member) in properties {
            let child = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            walk(member, &child, here, root, active_refs, out);
        }
    }
    if let Some(items) = node.get("items") {
        walk(items, &format!("{path}[]"), here, root, active_refs, out);
    }
    for combinator in ["allOf", "anyOf", "oneOf"] {
        if let Some(branches) = node.get(combinator).and_then(Value::as_array) {
            for branch in branches {
                walk(branch, path, here, root, active_refs, out);
            }
        }
    }
}

/// Every nesting fault in one schema.
fn faults(json: &str) -> Vec<String> {
    let root: Value = serde_json::from_str(json).expect("an embedded schema parses");
    let mut out = Vec::new();
    walk(&root, "", None, &root, &mut Vec::new(), &mut out);
    out
}

/// Faults left in a current schema on purpose, as (product group, version,
/// member path, why). Each must still be a fault, so the list cannot outlive what
/// it excuses.
const KNOWN_FAULTS: &[(&str, &str, &str, &str)] = &[
    // The three Annex XIII point 2(a) arrays share `materialComposition`, whose
    // `name` and `casNumber` are public. They cannot simply be relabelled
    // restricted: a definition's members are also recorded under their bare
    // name as a fail-closed floor, and `criticalRawMaterial` declares a public
    // `name` and `casNumber` in the same schema, so the two would collide and
    // the public pair would resolve to restricted. The fix renames the members
    // or changes that floor, which is its own change.
    (
        "battery",
        "2.8.0",
        "anodeMaterial[].name",
        "shared-definition floor",
    ),
    (
        "battery",
        "2.8.0",
        "anodeMaterial[].casNumber",
        "shared-definition floor",
    ),
    (
        "battery",
        "2.8.0",
        "cathodeMaterial[].name",
        "shared-definition floor",
    ),
    (
        "battery",
        "2.8.0",
        "cathodeMaterial[].casNumber",
        "shared-definition floor",
    ),
    (
        "battery",
        "2.8.0",
        "electrolyteMaterial[].name",
        "shared-definition floor",
    ),
    (
        "battery",
        "2.8.0",
        "electrolyteMaterial[].casNumber",
        "shared-definition floor",
    ),
    (
        "electronics",
        "1.4.0",
        "criticalRawMaterials[].name",
        "which critical raw materials a product contains is a disclosure question \
         under Regulation (EU) 2024/1252, read in #314; relabel it once that \
         reading is done",
    ),
    (
        "electronics",
        "1.4.0",
        "criticalRawMaterials[].countryOfOrigin",
        "as above: Regulation (EU) 2024/1252, read in #314",
    ),
];

/// The rule binds the schema a new passport is written against. A released
/// version is never edited, so what an earlier one got wrong stays wrong in it,
/// and the version that fixes it is the current one.
#[test]
fn every_member_of_a_current_schema_nests_within_its_enclosing_class() {
    let catalog = crate::catalog::ProductGroupCatalog::new();
    let mut found: Vec<(String, String, String)> = Vec::new();
    for schema in EMBEDDED {
        let current = catalog.resolve_schema_version(schema.product_group, None);
        if current.as_deref() != Some(schema.version) {
            continue;
        }
        for fault in faults(schema.json) {
            found.push((
                schema.product_group.to_owned(),
                schema.version.to_owned(),
                fault,
            ));
        }
    }

    let is_known = |group: &str, version: &str, fault: &str| {
        KNOWN_FAULTS
            .iter()
            .any(|(g, v, p, _)| *g == group && *v == version && fault.starts_with(&format!("{p} ")))
    };
    let unexcused: Vec<String> = found
        .iter()
        .filter(|(g, v, f)| !is_known(g, v, f))
        .map(|(g, v, f)| format!("  {g} v{v}: {f}"))
        .collect();
    assert!(
        unexcused.is_empty(),
        "members visible to an audience their enclosing object is hidden from:\n{}",
        unexcused.join("\n")
    );

    let stale: Vec<String> = KNOWN_FAULTS
        .iter()
        .filter(|(g, v, p, _)| {
            !found
                .iter()
                .any(|(fg, fv, f)| fg == g && fv == v && f.starts_with(&format!("{p} ")))
        })
        .map(|(g, v, p, _)| format!("  {g} v{v}: {p}"))
        .collect();
    assert!(
        stale.is_empty(),
        "KNOWN_FAULTS excuses what is no longer a fault, or a version that is no \
         longer current; remove these:\n{}",
        stale.join("\n")
    );
}

/// The check is only worth having if it catches the shape it is for, and passes
/// the shapes it is not.
#[test]
fn the_nesting_check_catches_what_it_is_for() {
    let schema = |parent: &str, member: &str| {
        serde_json::json!({
            "properties": {
                "block": {
                    "x-disclosure": parent,
                    "properties": { "value": { "x-disclosure": member } }
                }
            }
        })
        .to_string()
    };

    // A member more open than its parent, in each direction the lattice allows.
    for (parent, member) in [
        ("individual", "public"),
        ("restricted", "public"),
        ("individual", "restricted"),
        ("conformity", "individual"),
    ] {
        assert_eq!(
            faults(&schema(parent, member)).len(),
            1,
            "{member} inside {parent} must be a fault"
        );
    }
    // A member as or more restrictive than its parent.
    for (parent, member) in [
        ("individual", "individual"),
        ("restricted", "conformity"),
        ("restricted", "individual"),
        ("public", "restricted"),
    ] {
        assert!(
            faults(&schema(parent, member)).is_empty(),
            "{member} inside {parent} is not a fault"
        );
    }

    // Through a `$ref`, and through array items.
    let through_ref = serde_json::json!({
        "properties": {
            "list": {
                "x-disclosure": "restricted",
                "items": { "$ref": "#/definitions/entry" }
            }
        },
        "definitions": {
            "entry": { "properties": { "name": { "x-disclosure": "public" } } }
        }
    })
    .to_string();
    assert_eq!(
        faults(&through_ref),
        ["list[].name is public inside list, which is restricted"]
    );
}
