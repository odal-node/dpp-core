//! [`registry_product_group`] — this workspace's product group, by the registry's code.

use dpp_domain::ProductGroup;

/// The registry's code for batteries.
///
/// 👁️ **Observed** in the registry's web client on 2026-10-14: the registration
/// screen preselects the group with this code, and the User Guide names
/// batteries as the first product group open for registration.
pub const PRODUCT_GROUP_BATTERIES: &str = "BATTERIES";

/// The registry's code for `group`, or `None` where none has been observed.
///
/// `None` is not a code to fall back from. The registry's interface offers
/// batteries alone, and no other group's code has been seen; filing a
/// registration under this workspace's own name for the group would name one
/// the registry does not list.
#[must_use]
pub fn registry_product_group(group: &ProductGroup) -> Option<&'static str> {
    match group {
        ProductGroup::Battery => Some(PRODUCT_GROUP_BATTERIES),
        _ => None,
    }
}
