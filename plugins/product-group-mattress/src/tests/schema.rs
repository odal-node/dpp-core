//! The plugin's copies of schema facts equal the schema's.

use serde_json::Value;

use crate::PRIMARY_MATERIALS;

/// The plugin's copy of the `primaryMaterial` enum equals the schema's.
///
/// Read from the schema file at test time, because a plugin cannot `include_str!`
/// across the crate boundary and ship, and a copy with nothing holding it to the
/// original is how a plugin ends up accepting what its own schema refuses.
#[test]
fn primary_materials_are_the_schemas() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/dpp-domain/schemas/mattress/v1.1.0.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let schema: Value = serde_json::from_str(&text).expect("the schema is JSON");
    let declared: Vec<&str> = schema["properties"]["primaryMaterial"]["enum"]
        .as_array()
        .expect("primaryMaterial declares an enum")
        .iter()
        .map(|v| v.as_str().expect("enum members are strings"))
        .collect();
    assert_eq!(PRIMARY_MATERIALS, declared.as_slice());
}
