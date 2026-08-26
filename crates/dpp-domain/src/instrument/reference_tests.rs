//! How a recorded instrument reference carries its basis.

use super::*;

#[test]
fn round_trips_camel_case() {
    let reference = InstrumentRef::from_catalog("battery-reg-2023-1542");
    let json = serde_json::to_value(&reference).expect("serialise");
    assert_eq!(json["instrument"], "battery-reg-2023-1542");
    assert_eq!(json["recorded"], "catalog");
    assert_eq!(
        serde_json::from_value::<InstrumentRef>(json).expect("deserialise"),
        reference
    );
}

#[test]
fn an_operator_may_record_an_act_the_catalog_does_not_model() {
    let reference = InstrumentRef::from_operator("espr-horizontal-repairability");
    assert_eq!(reference.recorded, RecordedBasis::Operator);
    // No catalog lookup happens, by design: the horizontal case is exactly
    // the one the catalog cannot answer.
    assert_eq!(reference.instrument, "espr-horizontal-repairability");
}

/// Guards the collision described in the type docs. Every key this type
/// emits must be absent from every product-group schema, because the access
/// filter classifies nested keys by name and would otherwise apply a
/// product-group field's disclosure class to an envelope field.
#[test]
fn no_emitted_key_collides_with_a_product_group_schema_field() {
    let json = serde_json::to_value(InstrumentRef::from_catalog("x")).expect("serialise");
    let keys: Vec<&String> = json.as_object().expect("object").keys().collect();
    assert_eq!(keys.len(), 2, "update this test when the shape changes");

    let registry = crate::schemas::VersionedSchemaRegistry::new();
    for key in keys {
        for product_group in registry.product_groups() {
            let (_, schema) = registry.latest(product_group).expect("a schema");
            assert!(
                !schema.contains(&format!("\"{key}\"")),
                "'{key}' also appears in the {product_group} schema; the access filter \
                 matches nested keys by name, so that product group's disclosure class \
                 would be applied to this envelope field"
            );
        }
    }
}
