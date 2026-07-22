//! Host/guest ABI contract for Odal Node sector plugins.
//!
//! Plugins implement [`DppSectorPlugin`] and export the three entry points
//! as `extern "C"` symbols. The host invokes them through the wasmtime
//! component model or directly via the low-level ABI defined below.
//!
//! Data crosses the host/guest boundary as JSON strings over a shared-memory
//! slice, so the low-level ABI itself is just integer pointer/length pairs.
//! (The crate uses `std` types — `String`, `Vec`, `HashMap` — so it is not
//! `no_std`.)
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
//! - `plugin` — [`DppSectorPlugin`], the trait a plugin author implements,
//!   and [`PluginIdentity`], the per-plugin fields its default `meta()` uses.

mod error;
mod meta;
mod plugin;
mod result;
#[cfg(test)]
mod tests;
mod version;

pub use error::{PluginError, PluginFieldError};
pub use meta::{PluginCapabilities, PluginCapability, PluginMeta};
pub use plugin::{DppSectorPlugin, PluginIdentity, PluginInput};
pub use result::{
    AbiResult, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
    PluginComplianceStatus, PluginFinding, PluginResult,
};
pub use version::{
    ABI_VERSION_MAJOR, ABI_VERSION_MINOR, AbiVersion, CompatibilityStatus, SchemaVersionRange,
    check_compatibility,
};
