//! [`DppSectorPlugin`] — the trait every sector plugin implements.

use crate::error::PluginError;
use crate::meta::{PluginCapabilities, PluginMeta};
use crate::result::PluginResult;

/// Raw JSON input passed from the host to a plugin entry point.
pub type PluginInput = serde_json::Value;

/// The entry points every sector plugin must export.
///
/// The Wasm host calls these after deserialising JSON sector data from the
/// passport payload. Implementations must be deterministic and free of I/O.
pub trait DppSectorPlugin: Send + Sync {
    /// Returns static metadata about this plugin.
    fn meta(&self) -> PluginMeta;

    /// Returns the plugin's capability declaration for version negotiation.
    fn capabilities(&self) -> PluginCapabilities;

    /// Validate the structure and field constraints of the sector input.
    ///
    /// Returns `Ok(())` if the input is structurally valid, or a descriptive
    /// error if a required field is missing or out of range. Prefer
    /// `PluginError::ValidationErrors` with per-field detail over
    /// `PluginError::InvalidInput` for better error reporting.
    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError>;

    /// Compute compliance metrics from the sector input.
    ///
    /// May return `None` for fields that do not apply to this sector.
    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError>;

    /// Generate a passport-ready sector data JSON payload.
    ///
    /// Applies any normalisation or enrichment required by the sector schema
    /// (e.g. rounding, unit conversion). The output is stored verbatim in the DPP.
    fn generate_passport(&self, input: &PluginInput) -> Result<serde_json::Value, PluginError>;
}
