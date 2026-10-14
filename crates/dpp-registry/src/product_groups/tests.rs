//! The registry's product-group codes.

use dpp_domain::ProductGroup;

use super::{PRODUCT_GROUP_BATTERIES, registry_product_group};

#[test]
fn batteries_are_named_by_the_registry_s_code_not_ours() {
    assert_eq!(
        registry_product_group(&ProductGroup::Battery),
        Some(PRODUCT_GROUP_BATTERIES)
    );
    assert_ne!(
        PRODUCT_GROUP_BATTERIES,
        ProductGroup::Battery.wire_str(),
        "the registry's code and this workspace's name differ, which is why the mapping exists"
    );
}

/// No other group's code has been seen, and none is invented.
#[test]
fn a_group_the_registry_does_not_list_has_no_code() {
    for group in [
        ProductGroup::Textile,
        ProductGroup::Steel,
        ProductGroup::Other("detergents-2031".into()),
    ] {
        assert_eq!(registry_product_group(&group), None, "{group:?}");
    }
}
