//! [`DppSectorPlugin`] — the trait every sector plugin implements.

use crate::error::PluginError;
use crate::meta::{PluginCapabilities, PluginCapability, PluginMeta};
use crate::result::PluginResult;
use crate::version::{AbiVersion, SchemaVersionRange};

/// Raw JSON input passed from the host to a plugin entry point.
pub type PluginInput = serde_json::Value;

/// The identity fields that genuinely vary per plugin — everything else in
/// [`PluginMeta`] (`license`, `author`, `homepage`) is a fixed Odal Node
/// convention supplied by the default [`DppSectorPlugin::meta`].
pub struct PluginIdentity {
    /// Sector key this plugin handles, e.g. `"textile"`, `"steel"`, `"battery"`.
    pub sector: &'static str,
    /// Human-readable plugin name.
    pub name: &'static str,
    /// SemVer version string of the plugin itself, typically
    /// `env!("CARGO_PKG_VERSION")`.
    pub version: &'static str,
    /// Brief description of what this plugin does.
    pub description: &'static str,
}

/// The entry points every sector plugin must export.
///
/// The Wasm host calls these after deserialising JSON sector data from the
/// passport payload. Implementations must be deterministic and free of I/O.
pub trait DppSectorPlugin: Send + Sync {
    /// The fields that distinguish this plugin's identity (see [`PluginIdentity`]).
    fn plugin_identity(&self) -> PluginIdentity;

    /// The sector schema version range this plugin supports.
    fn schema_version_range(&self) -> SchemaVersionRange;

    /// Returns static metadata about this plugin.
    ///
    /// Built from [`Self::plugin_identity`] plus the fixed Odal Node
    /// `license`/`author`/`homepage` convention. Override directly if a
    /// plugin genuinely needs different values.
    fn meta(&self) -> PluginMeta {
        let id = self.plugin_identity();
        PluginMeta {
            sector: id.sector.to_owned(),
            name: id.name.to_owned(),
            version: id.version.to_owned(),
            license: "Apache-2.0".to_owned(),
            description: Some(id.description.to_owned()),
            author: Some("Odal Node".to_owned()),
            homepage: Some("https://github.com/odal-node/dpp-core".to_owned()),
        }
    }

    /// Returns the plugin's capability declaration for version negotiation.
    ///
    /// Built from [`Self::schema_version_range`] plus the standard
    /// `Validate`/`ComputeMetrics`/`GeneratePassport` capability set every
    /// plugin declares today. Override directly if a plugin needs a
    /// different capability set or negotiation limits.
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            abi_version: AbiVersion::current(),
            supported_schemas: vec![self.schema_version_range()],
            capabilities: vec![
                PluginCapability::Validate,
                PluginCapability::ComputeMetrics,
                PluginCapability::GeneratePassport,
            ],
            min_host_version: None,
            max_fuel: None,
            max_memory_bytes: None,
        }
    }

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
    /// Takes `input` by value — a pass-through implementation returns it
    /// directly instead of cloning.
    fn generate_passport(&self, input: PluginInput) -> Result<serde_json::Value, PluginError>;
}
