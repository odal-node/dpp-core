//! Host/guest ABI contract for Odal Node sector plugins.
//!
//! Plugins implement [`DppSectorPlugin`] and export the three entry points
//! as `extern "C"` symbols. The host invokes them through the wasmtime
//! component model or directly via the low-level ABI defined below.
//!
//! The interface is intentionally `no_std`-friendly: no heap allocations
//! are required from the host's perspective. Data is passed as JSON strings
//! over a shared-memory slice.
//!
//! ## Versioning
//!
//! Every plugin declares which ABI version and schema versions it supports
//! via [`PluginCapabilities`]. The host uses this for compatibility checks
//! before dispatching any calls.
//!
//! ## Module layout
//!
//! - `version` — [`AbiVersion`], [`SchemaVersionRange`], [`CompatibilityStatus`],
//!   and [`check_compatibility`] (ABI/schema/capability negotiation).
//! - `meta` — [`PluginMeta`], [`PluginCapability`], [`PluginCapabilities`].
//! - `result` — [`PluginComplianceStatus`], [`PluginFinding`], [`PluginResult`],
//!   [`AbiResult`] (the call-outcome envelope).
//! - `error` — [`PluginError`], [`PluginFieldError`].
//! - `plugin` — [`DppSectorPlugin`], the trait a plugin author implements.

mod error;
mod meta;
mod plugin;
mod result;
#[cfg(test)]
mod tests;
mod version;

pub use error::{PluginError, PluginFieldError};
pub use meta::{PluginCapabilities, PluginCapability, PluginMeta};
pub use plugin::{DppSectorPlugin, PluginInput};
pub use result::{
    AbiResult, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
    PluginComplianceStatus, PluginFinding, PluginResult,
};
pub use version::{
    ABI_VERSION_MAJOR, ABI_VERSION_MINOR, AbiVersion, CompatibilityStatus, SchemaVersionRange,
    check_compatibility,
};
