//! Access tier filter engine — applies a `SectorAccessPolicy` to a JSON document.

use dpp_domain::AccessTier;

use super::policy::SectorAccessPolicy;

/// The result of a policy evaluation.
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// The caller's effective access tier.
    pub granted_tier: AccessTier,
    /// Fields that were redacted (not visible to this caller).
    pub redacted_fields: Vec<String>,
    /// The filtered JSON document.
    pub filtered_data: serde_json::Value,
}

/// Filter a JSON document according to the sector access policy and caller's tier.
///
/// **Path-aware and recursive:** every key — at every nesting depth, including
/// inside arrays of objects — is classified by [`SectorAccessPolicy::tier_for_field`]
/// and removed when its tier exceeds the caller's. A field kept at one level is
/// still descended into, so a Confidential field nested inside an otherwise-public
/// object cannot leak. Redacted keys are reported as dotted paths
/// (e.g. `sectorData.svhcSubstances`, `criticalRawMaterials[0].casNumber`).
///
/// Non-object/array inputs are returned unchanged.
pub fn filter_by_access_tier(
    data: &serde_json::Value,
    policy: &SectorAccessPolicy,
    caller_tier: AccessTier,
) -> PolicyDecision {
    let mut redacted_fields = Vec::new();
    let filtered_data = filter_value(data, policy, caller_tier, "", &mut redacted_fields);
    PolicyDecision {
        granted_tier: caller_tier,
        redacted_fields,
        filtered_data,
    }
}

fn filter_value(
    data: &serde_json::Value,
    policy: &SectorAccessPolicy,
    caller_tier: AccessTier,
    prefix: &str,
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
                if caller_tier >= policy.tier_for_field(key) {
                    filtered.insert(
                        key.clone(),
                        filter_value(value, policy, caller_tier, &path, redacted),
                    );
                } else {
                    redacted.push(path);
                }
            }
            serde_json::Value::Object(filtered)
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    filter_value(v, policy, caller_tier, &format!("{prefix}[{i}]"), redacted)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
