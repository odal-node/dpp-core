//! [`RegistryRecord`] — a confirmed registration returned by the registry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::identifiers::RegistryIdentifiers;
use super::status::RegistryStatus;

/// A confirmed registration record returned by the EU Central Registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRecord {
    /// The four persistent identifiers assigned by the registry.
    pub identifiers: RegistryIdentifiers,
    /// Current status of this registration.
    pub status: RegistryStatus,
    /// Timestamp when the registration was confirmed.
    pub registered_at: DateTime<Utc>,
    /// Timestamp of the most recent status change.
    pub updated_at: DateTime<Utc>,
}
