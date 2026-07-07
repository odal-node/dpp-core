//! Error types: identifier validation ([`RegistryValidationError`]) and
//! registry-operation errors ([`EuRegistryError`] / [`EuRegistryErrorKind`]).

use serde::{Deserialize, Serialize};

/// Error returned when a bridge identifier fails structural validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryValidationError {
    /// A GTIN value is structurally invalid (wrong length or failed mod-10).
    InvalidGtin { value: String, reason: String },
    /// A GLN facility value is structurally invalid (wrong length or failed mod-10).
    InvalidGln { value: String, reason: String },
    /// An operator identifier failed the structural/checksum check for its scheme.
    InvalidOperatorId { scheme: String, value: String },
    /// A country code is not a valid ISO 3166-1 alpha-2 code.
    InvalidCountryCode { code: String },
    /// A required payload field is empty.
    MissingRequiredField(String),
}

impl std::fmt::Display for RegistryValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidGtin { value, reason } => {
                write!(f, "invalid GTIN '{value}': {reason}")
            }
            Self::InvalidGln { value, reason } => {
                write!(f, "invalid GLN '{value}': {reason}")
            }
            Self::InvalidOperatorId { scheme, value } => {
                write!(f, "invalid {scheme} operator identifier '{value}'")
            }
            Self::InvalidCountryCode { code } => {
                write!(
                    f,
                    "invalid country code '{code}': must be an ISO 3166-1 alpha-2 code (2 uppercase ASCII letters)"
                )
            }
            Self::MissingRequiredField(field) => {
                write!(f, "required field '{field}' is empty")
            }
        }
    }
}

impl std::error::Error for RegistryValidationError {}

/// Error categories for EU registry operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EuRegistryErrorKind {
    /// Network or transport failure.
    ConnectionFailed,
    /// Authentication or authorisation failure.
    Unauthorized,
    /// The registry returned an unexpected response format.
    InvalidResponse,
    /// The registration was rejected by the registry.
    RegistrationRejected,
    /// Rate limit exceeded.
    RateLimited,
    /// The passport was not found in the registry.
    NotFound,
    /// The registry reported an internal error.
    RegistryInternalError,
    /// Request timed out.
    Timeout,
}

/// Error returned by EU registry operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EuRegistryError {
    pub kind: EuRegistryErrorKind,
    pub message: String,
    /// HTTP status code, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Registry-specific error code, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_error_code: Option<String>,
}

impl std::fmt::Display for EuRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EU Registry error ({:?}): {}", self.kind, self.message)
    }
}

impl std::error::Error for EuRegistryError {}
