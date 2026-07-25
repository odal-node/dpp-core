//! Access filter engine — applies a `SectorAccessPolicy` to a JSON document.

use dpp_domain::Audience;

use super::policy::SectorAccessPolicy;

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

/// Filter a JSON document according to the sector access policy and the
/// caller's audience.
///
/// **Path-aware and recursive:** every key — at every nesting depth, including
/// inside arrays of objects — is classified by [`SectorAccessPolicy::disclosure_for_field`]
/// and removed when [`Audience::may_see`] says that audience may not see its
/// class. A field kept at one level is still descended into, so a restricted
/// field nested inside an otherwise-public object cannot leak.
///
/// Visibility is a lattice, not a threshold: an `Authority` is not a superset of
/// a `LegitimateInterest` holder. Individual-item data (Annex XIII point 4) is
/// withheld from authorities, and conformity evidence (point 3) is withheld from
/// legitimate-interest holders. Redacted keys are reported as dotted paths
/// (e.g. `sectorData.svhcSubstances`, `criticalRawMaterials[0].casNumber`).
///
/// Non-object/array inputs are returned unchanged.
pub fn filter_by_audience(
    data: &serde_json::Value,
    policy: &SectorAccessPolicy,
    audience: Audience,
) -> PolicyDecision {
    let mut redacted_fields = Vec::new();
    let filtered_data = filter_value(data, policy, audience, "", &mut redacted_fields);
    PolicyDecision {
        audience,
        redacted_fields,
        filtered_data,
    }
}

fn filter_value(
    data: &serde_json::Value,
    policy: &SectorAccessPolicy,
    audience: Audience,
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
                if audience.may_see(policy.disclosure_for_field(key)) {
                    filtered.insert(
                        key.clone(),
                        filter_value(value, policy, audience, &path, redacted),
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
                    filter_value(v, policy, audience, &format!("{prefix}[{i}]"), redacted)
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
