//! `vct` minting — the identifier a credential is named by.

use dpp_domain::ProductGroup;

use super::vct::vct_for;
#[test]
fn a_vct_names_the_product_group_and_its_schema_version() {
    assert_eq!(
        vct_for(ProductGroup::Battery, "2.6.0"),
        "tag:odal-node.io,2026:vct:battery:2.6.0"
    );
}

#[test]
fn two_schema_versions_are_two_types() {
    assert_ne!(
        vct_for(ProductGroup::Battery, "2.6.0"),
        vct_for(ProductGroup::Battery, "2.5.0")
    );
}

#[test]
fn two_product_groups_are_two_types() {
    assert_ne!(
        vct_for(ProductGroup::Battery, "1.0.0"),
        vct_for(ProductGroup::Textile, "1.0.0")
    );
}

/// The identifier must not be an HTTPS URL, which is the whole point of the
/// choice above and the one property a later "convenience" change would
/// silently reverse.
#[test]
fn a_vct_is_not_dereferenceable() {
    let vct = vct_for(ProductGroup::Battery, "2.6.0");
    assert!(vct.starts_with("tag:"));
    assert!(!vct.contains("https://"));
    assert!(!vct.contains("http://"));
}
