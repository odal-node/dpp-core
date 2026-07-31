//! `PluginHost` port — delegates compliance computation to loaded Wasm sector plugins.

use crate::domain::sector::SectorData;
use crate::ports::compliance::{ComplianceError, ComplianceResult};

/// Port trait for the Wasm plugin host.
///
/// Implementors load sector-specific Wasm plugins and delegate compliance
/// computation to them. Tests can wire a stub that returns fixed results.
/// The passthrough registry is used when no plugin is loaded for a sector.
///
/// Dispatch is by **catalog key**, not by the `Sector` enum. Plugin manifests
/// are string-keyed and the catalog is the source of sector identity; taking
/// the enum here meant a plugin could only ever be loaded for a sector this
/// build already had a variant for, which is the opposite of what a plugin
/// host is for. Use [`Sector::catalog_key`](crate::domain::sector::Sector::catalog_key)
/// at the call site.
pub trait PluginHost: Send + Sync {
    /// Returns true if a Wasm plugin is currently loaded for `sector_key`.
    fn has_plugin(&self, sector_key: &str) -> bool;

    /// Invoke the loaded plugin for `sector_key` with the given `data`.
    ///
    /// Returns `ComplianceErrorKind::UnknownSector` if no plugin is loaded.
    fn compute(
        &self,
        sector_key: &str,
        data: &SectorData,
    ) -> Result<ComplianceResult, ComplianceError>;
}
