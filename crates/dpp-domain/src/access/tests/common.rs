//! Helpers the access tests share.

use crate::Audience;

use super::super::filter::filter_by_audience_in_scope;
use super::super::policy::{DocumentScope, ProductGroupAccessPolicy};

/// Filter a **bare product-group payload** — a document whose root is already
/// inside `productGroupData`, which is how the resolver filters it on its second
/// pass. Says so explicitly, because a payload filtered as an envelope would have
/// none of its product group's classes applied and would serve every restricted
/// field in it.
pub(super) fn filter_payload(
    data: &serde_json::Value,
    policy: &ProductGroupAccessPolicy,
    audience: Audience,
) -> super::super::filter::PolicyDecision {
    filter_by_audience_in_scope(data, policy, audience, DocumentScope::ProductGroupData)
}

/// The policy for a product group's current schema version — the path a served
/// passport actually takes.
pub(super) fn current_policy(product_group: &str) -> ProductGroupAccessPolicy {
    let reg = crate::schemas::VersionedSchemaRegistry::new();
    let (version, _) = reg
        .latest(product_group)
        .expect("product_group has a schema");
    ProductGroupAccessPolicy::for_schema_version(product_group, &version.to_string())
        .expect("current version yields a policy")
}

pub(super) fn battery_policy() -> ProductGroupAccessPolicy {
    current_policy("battery")
}
