//! [`RegistryRecord`] — a registered passport, as the registry reports it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::status::RegistryStatus;

/// A registered passport, as the EU Central Registry reports it.
///
/// Carries the one identifier the registry assigns. The others a registration
/// names — the product's, the operator's, the facility's — are supplied by the
/// operator and are already on the request; the registry generates only the
/// registration identifier (IR (EU) 2026/1778 Art. 8(8)).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRecord {
    /// The identifier the registry generated for this registration.
    pub registration_id: String,
    /// Current status of this registration.
    pub status: RegistryStatus,
    /// Timestamp when the registration was confirmed.
    pub registered_at: DateTime<Utc>,
    /// Timestamp of the most recent status change.
    pub updated_at: DateTime<Utc>,
}
