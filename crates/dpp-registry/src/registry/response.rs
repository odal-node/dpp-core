//! Registry query/registration responses: [`EuRegistryResponse`],
//! [`StatusResponse`], and their shared [`RegistryStatusCode`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status codes returned by the EU registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryStatusCode {
    /// Registration received and being processed.
    Pending,
    /// Successfully registered — DPP is live in the EU registry.
    Registered,
    /// Registration rejected — see `rejection_reasons`.
    Rejected,
    /// Registration suspended by a market surveillance authority.
    SuspendedByAuthority,
    /// DPP has been deactivated (product withdrawn from market).
    Deactivated,
}

/// Response from the EU registry after a registration or status query.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EuRegistryResponse {
    /// The registry's own unique ID for this DPP registration.
    pub registry_id: String,
    /// The passport ID that was registered.
    pub passport_id: Uuid,
    /// Current status in the registry.
    pub status: RegistryStatusCode,
    /// Human-readable status message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Rejection reasons, if status is `Rejected`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reasons: Option<Vec<String>>,
    /// Timestamp when the registry last updated this record.
    pub updated_at: DateTime<Utc>,
}

/// Simplified status query response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub registry_id: String,
    pub status: RegistryStatusCode,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
