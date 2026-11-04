//! The `ProductGroup` discriminant: wire keys and round-trips.

use super::group::*;

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

/// An untyped group answers no category values, even under a key a typed group
/// owns. Its payload's `product_category` is `None`, because nothing knows which
/// of its keys is a category, and a list here would contradict that.
#[test]
fn an_untyped_product_group_has_no_category_values() {
    for key in ["packaging", "battery"] {
        assert_eq!(
            ProductGroup::Other(key.into()).product_category_values(),
            None,
            "Other({key:?})"
        );
    }
}

/// The values are the schema's legal domain, not the catalog's curated list:
/// furniture's schema allows `bed`, which its catalog entry does not name.
#[test]
fn category_values_are_the_schema_domain_not_the_catalog_list() {
    let values = ProductGroup::Furniture
        .product_category_values()
        .expect("furniture has a category axis");
    assert!(values.iter().any(|v| v == "bed"), "{values:?}");

    let catalog = crate::catalog::ProductGroupCatalog::new();
    let listed = &catalog
        .get("furniture")
        .expect("furniture is catalogued")
        .product_categories;
    assert!(
        !listed.iter().any(|c| c == "bed"),
        "the catalog now lists `bed`; pin a value the schema allows and the catalog does not"
    );
}

/// A stored passport may state a category its group's current schema has
/// withdrawn, and only the any-version list carries it: electronics before
/// `v1.2.0` allowed `laptop`, and furniture `mattress`. Checked against the
/// current list, a value such a passport states would read as one no passport
/// can state.
#[test]
fn a_withdrawn_category_is_a_value_in_any_version_only() {
    for (group, withdrawn) in [
        (ProductGroup::Electronics, "laptop"),
        (ProductGroup::Furniture, "mattress"),
    ] {
        let current = group.product_category_values().expect("a category axis");
        let any = group
            .product_category_values_any_version()
            .expect("a category axis");
        assert!(
            !current.iter().any(|v| v == withdrawn),
            "{group:?} allows `{withdrawn}` again; pin a value it has withdrawn: {current:?}"
        );
        assert!(
            any.iter().any(|v| v == withdrawn),
            "{group:?} under any version: {any:?}"
        );
    }
}

/// The any-version list begins with the current one, holds each value once,
/// and is absent for exactly the groups the current list is absent for.
#[test]
fn the_any_version_list_extends_the_current_one() {
    for group in [
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
        match (
            group.product_category_values(),
            group.product_category_values_any_version(),
        ) {
            (None, None) => {}
            (Some(current), Some(any)) => {
                assert_eq!(any.get(..current.len()), Some(current), "{group:?}");
                let mut seen = std::collections::BTreeSet::new();
                assert!(
                    any.iter().all(|v| seen.insert(v)),
                    "{group:?} lists a value twice: {any:?}"
                );
            }
            (current, any) => panic!("{group:?}: current {current:?}, any version {any:?}"),
        }
    }
}
