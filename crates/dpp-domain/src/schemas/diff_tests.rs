//! What [`diff_schemas`](super::diff::diff_schemas) reports, and how it classifies it.

use serde_json::json;

use super::change::ChangeKind;
use super::diff::diff_schemas;

/// Two versions differing by one added property.
fn added_property() -> (serde_json::Value, serde_json::Value) {
    (
        json!({
            "type": "object",
            "required": ["gtin"],
            "properties": { "gtin": { "type": "string", "pattern": "^[0-9]{14}$" } }
        }),
        json!({
            "type": "object",
            "required": ["gtin"],
            "properties": {
                "gtin": { "type": "string", "pattern": "^[0-9]{14}$" },
                "recycledContentPct": { "type": "number", "minimum": 0.0 }
            }
        }),
    )
}

#[test]
fn an_added_property_is_named_and_classified_as_an_addition() {
    let (prev, next) = added_property();
    let diff = diff_schemas(&prev, &next);

    assert_eq!(diff.properties.len(), 1);
    assert_eq!(diff.properties[0].path, "recycledContentPct");
    assert_eq!(diff.properties[0].kind, ChangeKind::Added);
    assert_eq!(diff.properties[0].after, "number");
    assert!(diff.is_purely_additive());
}

/// The distinction the report exists to draw: a tightened pattern is not an
/// addition, and reading it as one is how a narrowing change gets waved through.
#[test]
fn a_changed_pattern_is_a_constraint_change_not_an_addition() {
    let prev = json!({
        "properties": { "countryOfOrigin": { "type": "string", "pattern": "^[A-Z]{2}$" } }
    });
    let next = json!({
        "properties": { "countryOfOrigin": { "type": "string", "pattern": "^(DE|FR)$" } }
    });

    let diff = diff_schemas(&prev, &next);
    assert_eq!(diff.properties.len(), 1);
    assert_eq!(diff.properties[0].kind, ChangeKind::ConstraintChanged);
    assert_eq!(diff.properties[0].path, "countryOfOrigin");
    assert!(diff.properties[0].before.contains("^[A-Z]{2}$"));
    assert!(diff.properties[0].after.contains("^(DE|FR)$"));
    assert!(!diff.is_purely_additive());
}

#[test]
fn a_changed_type_is_reported_as_a_type_change() {
    let prev = json!({ "properties": { "weight": { "type": "string" } } });
    let next = json!({ "properties": { "weight": { "type": "number" } } });

    let diff = diff_schemas(&prev, &next);
    assert_eq!(diff.properties.len(), 1);
    assert_eq!(diff.properties[0].kind, ChangeKind::TypeChanged);
    assert_eq!(diff.properties[0].before, "string");
    assert_eq!(diff.properties[0].after, "number");
}

#[test]
fn a_removed_property_is_reported() {
    let (next, prev) = added_property(); // reversed
    let diff = diff_schemas(&prev, &next);
    assert_eq!(diff.properties.len(), 1);
    assert_eq!(diff.properties[0].kind, ChangeKind::Removed);
    assert_eq!(diff.properties[0].path, "recycledContentPct");
}

/// A field added inside a sub-object must not read as a change to the object.
#[test]
fn a_nested_addition_is_reported_at_its_full_path() {
    let prev = json!({
        "properties": {
            "stateOfHealth": { "type": "object", "properties": { "pct": { "type": "number" } } }
        }
    });
    let next = json!({
        "properties": {
            "stateOfHealth": {
                "type": "object",
                "properties": {
                    "pct": { "type": "number" },
                    "measuredAt": { "type": "string", "format": "date-time" }
                }
            }
        }
    });

    let diff = diff_schemas(&prev, &next);
    assert_eq!(diff.properties.len(), 1);
    assert_eq!(diff.properties[0].path, "stateOfHealth/measuredAt");
    assert_eq!(diff.properties[0].kind, ChangeKind::Added);
}

/// Array element schemas are walked too, under a `[]` segment.
#[test]
fn an_addition_inside_array_items_is_reported() {
    let prev = json!({
        "properties": {
            "materials": {
                "type": "array",
                "items": { "type": "object", "properties": { "name": { "type": "string" } } }
            }
        }
    });
    let next = json!({
        "properties": {
            "materials": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "weightKg": { "type": "number" }
                    }
                }
            }
        }
    });

    let diff = diff_schemas(&prev, &next);
    assert_eq!(diff.properties.len(), 1);
    assert_eq!(diff.properties[0].path, "materials/[]/weightKg");
}

#[test]
fn a_newly_required_property_is_reported_and_is_not_additive() {
    let prev = json!({ "required": ["gtin"], "properties": { "gtin": { "type": "string" } } });
    let next = json!({
        "required": ["gtin", "batchId"],
        "properties": { "gtin": { "type": "string" }, "batchId": { "type": "string" } }
    });

    let diff = diff_schemas(&prev, &next);
    assert_eq!(diff.required_added, vec!["batchId".to_owned()]);
    assert!(diff.required_removed.is_empty());
    assert!(
        !diff.is_purely_additive(),
        "adding a property and requiring it are different changes, and the \
         second is not additive for a document that omits it"
    );
}

#[test]
fn identical_schemas_produce_an_empty_diff() {
    let (prev, _) = added_property();
    let diff = diff_schemas(&prev, &prev);
    assert!(diff.is_empty());
    assert!(diff.is_purely_additive(), "an empty diff adds nothing");
}

/// A requirement inside an array's items binds every row, so dropping one is a
/// change of obligation. Only the root's `required` used to be compared, which made
/// this diff empty and its bump "additive".
#[test]
fn a_requirement_dropped_inside_array_items_is_reported_at_its_path() {
    let rows = |required: serde_json::Value| {
        json!({
            "type": "object",
            "properties": {
                "lines": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": required,
                        "properties": {
                            "description": { "type": "string" },
                            "reason": { "type": "string" }
                        }
                    }
                }
            }
        })
    };

    let diff = diff_schemas(
        &rows(json!(["description", "reason"])),
        &rows(json!(["description"])),
    );
    assert_eq!(diff.required_removed, vec!["lines/[]/reason".to_owned()]);
    assert!(diff.required_added.is_empty());
    assert!(!diff.is_empty());
    assert!(!diff.is_purely_additive());
}

/// The same inside a sub-object, and a requirement added there is not additive.
#[test]
fn a_requirement_added_inside_a_sub_object_is_reported_at_its_path() {
    let entity = |required: serde_json::Value| {
        json!({
            "type": "object",
            "properties": {
                "entity": {
                    "type": "object",
                    "required": required,
                    "properties": { "name": { "type": "string" }, "id": { "type": "string" } }
                }
            }
        })
    };

    let diff = diff_schemas(&entity(json!(["name"])), &entity(json!(["name", "id"])));
    assert_eq!(diff.required_added, vec!["entity/id".to_owned()]);
    assert!(!diff.is_purely_additive());
}
