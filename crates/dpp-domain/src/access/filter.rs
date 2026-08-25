//! Access filter engine — applies a `ProductGroupAccessPolicy` to a JSON document.

use crate::Audience;

use super::policy::{DocumentScope, ProductGroupAccessPolicy};

/// The result of a policy evaluation.
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// The audience the document was filtered for.
    pub audience: Audience,
    /// Fields that were redacted (not visible to this caller).
    pub redacted_fields: Vec<String>,
    /// The filtered JSON document.
    pub filtered_data: serde_json::Value,
}

/// Filter a JSON document according to the product group access policy and the
/// caller's audience.
///
/// **Scope- and depth-aware, recursive:** every key — at every nesting depth,
/// including inside arrays of objects — is classified by
/// [`ProductGroupAccessPolicy::disclosure_for_key`] and removed when
/// [`Audience::may_see`] says that audience may not see its class. A field kept
/// at one level is still descended into, so a restricted field nested inside an
/// otherwise-public object cannot leak.
///
/// Which classes apply depends on **where** a key sits: a product group's schema
/// governs its own payload and stops there, while envelope classes apply
/// everywhere. See [`DocumentScope`] for why that boundary exists and what went
/// wrong without it. Matching *within* a scope is still by leaf name, so a
/// generic name is still a poor thing to restrict — that limit is unchanged.
///
/// Visibility is a lattice, not a threshold: an `Authority` is not a superset of
/// a `LegitimateInterest` holder. Individual-item data (Annex XIII point 4) is
/// withheld from authorities, and conformity evidence (point 3) is withheld from
/// legitimate-interest holders. Redacted keys are reported as dotted paths
/// (e.g. `productGroupData.svhcSubstances`, `criticalRawMaterials[0].casNumber`).
///
/// Non-object/array inputs are returned unchanged.
/// Filters a **whole passport** — a document whose root is the envelope, with
/// the product group's payload under `productGroupData`.
///
/// For a bare product-group payload handed in as its own root document — which
/// the resolver does, filtering the envelope and the payload in two passes —
/// use [`filter_by_audience_in_scope`] and say so. Getting that wrong is not a
/// cosmetic error: a payload filtered as an envelope has none of its product
/// group's classes applied, so every restricted field in it would be served.
pub fn filter_by_audience(
    data: &serde_json::Value,
    policy: &ProductGroupAccessPolicy,
    audience: Audience,
) -> PolicyDecision {
    filter_by_audience_in_scope(data, policy, audience, DocumentScope::Envelope)
}

/// Filter a document whose root sits in `root_scope`.
///
/// The scope cannot be inferred from the document: a bare product-group payload
/// and a passport envelope are both JSON objects, and only the caller knows
/// which it is holding. Passing the wrong one fails in the unsafe direction for
/// a payload — see [`filter_by_audience`].
pub fn filter_by_audience_in_scope(
    data: &serde_json::Value,
    policy: &ProductGroupAccessPolicy,
    audience: Audience,
    root_scope: DocumentScope,
) -> PolicyDecision {
    let mut redacted_fields = Vec::new();
    let filtered_data = filter_value(data, policy, audience, "", root_scope, &mut redacted_fields);
    PolicyDecision {
        audience,
        redacted_fields,
        filtered_data,
    }
}

/// The envelope key below which a product group's own schema governs.
const PRODUCT_GROUP_DATA: &str = "productGroupData";

fn filter_value(
    data: &serde_json::Value,
    policy: &ProductGroupAccessPolicy,
    audience: Audience,
    prefix: &str,
    scope: DocumentScope,
    redacted: &mut Vec<String>,
) -> serde_json::Value {
    match data {
        serde_json::Value::Object(map) => {
            let mut filtered = serde_json::Map::new();
            for (key, value) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                // The key itself is classified in the scope it *sits* in; its
                // children are classified in the scope it *opens*. Crossing at
                // the child rather than the key is what keeps `productGroupData`
                // an envelope field — it is the passport's, not the product
                // group's, and a schema must not be able to reclassify the
                // container it is carried in.
                let child_scope = if scope == DocumentScope::ProductGroupData
                    || key.as_str() == PRODUCT_GROUP_DATA
                {
                    DocumentScope::ProductGroupData
                } else {
                    DocumentScope::Envelope
                };
                if audience.may_see(policy.disclosure_for_key(key, scope)) {
                    filtered.insert(
                        key.clone(),
                        filter_value(value, policy, audience, &path, child_scope, redacted),
                    );
                } else {
                    redacted.push(path);
                }
            }
            serde_json::Value::Object(filtered)
        }
        // An array does not change scope: its items are the same thing its key
        // was, indexed.
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    filter_value(
                        v,
                        policy,
                        audience,
                        &format!("{prefix}[{i}]"),
                        scope,
                        redacted,
                    )
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
