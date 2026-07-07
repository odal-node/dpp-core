//! Plugin error types: [`PluginError`] and its field-level detail [`PluginFieldError`].

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Structured error with field-level detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFieldError {
    /// JSON pointer to the failing field, e.g. `"/fibreComposition/0/pct"`.
    pub field: String,
    /// Error code for programmatic handling (e.g. `"out_of_range"`, `"missing"`).
    pub code: String,
    /// Human-readable error message.
    pub message: String,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum PluginError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("validation errors: {0:?}")]
    ValidationErrors(Vec<PluginFieldError>),
    #[error("calculation failed: {0}")]
    Calculation(String),
    #[error("sector not supported by this plugin: {0}")]
    UnsupportedSector(String),
    #[error("schema version not supported: {0}")]
    UnsupportedSchemaVersion(String),
    #[error("capability not available: {0}")]
    CapabilityNotAvailable(String),
    #[error("internal plugin error: {0}")]
    Internal(String),
}
