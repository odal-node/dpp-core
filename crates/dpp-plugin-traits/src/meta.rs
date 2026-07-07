//! Plugin identity ([`PluginMeta`]) and capability declaration
//! ([`PluginCapability`] / [`PluginCapabilities`]).

use serde::{Deserialize, Serialize};

use crate::version::{AbiVersion, SchemaVersionRange};

/// Static metadata returned by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMeta {
    /// Sector key this plugin handles, e.g. `"textile"`, `"steel"`, `"battery"`.
    pub sector: String,
    /// Human-readable plugin name.
    pub name: String,
    /// SemVer version string of the plugin itself.
    pub version: String,
    /// SPDX license identifier.
    pub license: String,
    /// Brief description of what this plugin does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Plugin author or organisation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// URL for plugin documentation or source code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

/// Feature flags a plugin may declare support for.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Can validate sector-specific data against the schema.
    Validate,
    /// Can compute compliance metrics (CO2e, repairability, etc.).
    ComputeMetrics,
    /// Can generate a passport-ready data payload.
    GeneratePassport,
    /// Can perform SVHC / substance-of-concern screening.
    SubstanceScreening,
    /// Can compute lifecycle assessment (LCA) metrics.
    LifecycleAssessment,
    /// Can map data to Asset Administration Shell (AAS) submodels.
    AasMapping,
    /// Custom capability (plugin-defined extension point).
    Custom(String),
}

/// Full capability declaration returned by a plugin during negotiation.
///
/// The host calls `capabilities()` before dispatching any work to verify
/// that the plugin supports the required schema version and features.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilities {
    /// The ABI version this plugin was compiled against.
    pub abi_version: AbiVersion,
    /// The sector schemas this plugin can handle.
    pub supported_schemas: Vec<SchemaVersionRange>,
    /// Feature capabilities this plugin provides.
    pub capabilities: Vec<PluginCapability>,
    /// Minimum host ABI version required by this plugin.
    /// If the host's ABI is below this, the plugin refuses to load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_host_version: Option<AbiVersion>,
    /// Plugin-declared fuel budget per invocation (host caps at DEFAULT_FUEL).
    /// Plugins needing less computation can set this lower for tighter sandboxing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fuel: Option<u64>,
    /// Plugin-declared memory cap in bytes per invocation (host caps at DEFAULT_MEMORY_CAP_BYTES).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory_bytes: Option<u64>,
}
