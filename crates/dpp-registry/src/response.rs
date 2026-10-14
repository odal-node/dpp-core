//! Registry query/registration responses: [`EuRegistryResponse`],
//! [`StatusResponse`], and their shared [`RegistryStatusCode`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status codes returned by the EU registry.
///
/// ⚠️ **An unverified vocabulary**, and so an open one. The registry publishes
/// no record-status list, and IR (EU) 2026/1778 implies states none of these
/// names: **Art. 4(4)** has the status of a passport *"updated accordingly"*
/// when its operator's verification lapses, and **Art. 10(1)** reflects every
/// creation, modification and deletion in *"the status of the registration"*.
/// A value outside the five below deserialises as
/// [`Unrecognised`](Self::Unrecognised), carrying the value, rather than
/// failing the whole reply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
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
    /// A value outside the five above, exactly as the registry sent it.
    ///
    /// A caller must not map it onto one of the others. Read as `Rejected` it
    /// sends an operator to fix a submission that may be fine; read as
    /// `Registered` it reports a record that may be suspended.
    Unrecognised(String),
}

impl RegistryStatusCode {
    /// The wire value, spelled out rather than derived from `Serialize`, so a
    /// renamed variant cannot silently change what is matched.
    #[must_use]
    pub fn wire_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Registered => "registered",
            Self::Rejected => "rejected",
            Self::SuspendedByAuthority => "suspended_by_authority",
            Self::Deactivated => "deactivated",
            Self::Unrecognised(value) => value,
        }
    }
}

impl From<String> for RegistryStatusCode {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pending" => Self::Pending,
            "registered" => Self::Registered,
            "rejected" => Self::Rejected,
            "suspended_by_authority" => Self::SuspendedByAuthority,
            "deactivated" => Self::Deactivated,
            _ => Self::Unrecognised(value),
        }
    }
}

impl From<RegistryStatusCode> for String {
    fn from(status: RegistryStatusCode) -> Self {
        match status {
            RegistryStatusCode::Unrecognised(value) => value,
            known => known.wire_str().to_owned(),
        }
    }
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
