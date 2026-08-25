//! The `ProductGroup` discriminant: wire keys and round-trips.

use super::product_group::*;

#[test]
fn wire_str_matches_serde_serialization() {
    for product_group in [
        ProductGroup::Battery,
        ProductGroup::Textile,
        ProductGroup::UnsoldGoods,
        ProductGroup::Steel,
        ProductGroup::Electronics,
        ProductGroup::Construction,
        ProductGroup::Tyre,
        ProductGroup::Toy,
        ProductGroup::Aluminium,
        ProductGroup::Furniture,
        ProductGroup::Mattress,
        ProductGroup::Detergent,
        ProductGroup::Other("packaging".into()),
    ] {
        let serialized = serde_json::to_value(&product_group).unwrap();
        assert_eq!(
            serialized.as_str().unwrap(),
            product_group.wire_str(),
            "wire_str() disagrees with serde for {product_group:?}"
        );
    }
}
