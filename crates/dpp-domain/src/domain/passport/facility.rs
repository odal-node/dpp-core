//! [`FacilitySnapshot`] — a by-value copy of the Annex III facility at passport creation.

use serde::{Deserialize, Serialize};

/// A self-contained snapshot of the ESPR Annex III facility a passport was
/// stamped with at creation.
///
/// Copied **by value** onto the passport so the published, signed record remains
/// complete for its full retention period even if the operator later retires the
/// source facility from their registry. Field shape matches
/// `dpp_registry::FacilityIdentifier`. See DATA-MODEL §3.3 / ADR-006.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacilitySnapshot {
    /// Identifier scheme (e.g. `"gln"`, `"national"`).
    pub scheme: String,
    /// Identifier value (e.g. the 13-digit GLN) — the Annex III unique facility id.
    pub value: String,
    /// Human-readable facility name.
    pub name: String,
    /// ISO 3166-1 alpha-2 country code of the facility.
    pub country: String,
    /// Optional street address / location description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}
