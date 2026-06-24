//! `PluginHost` port — delegates compliance computation to loaded Wasm sector plugins.

use crate::domain::sector::{Sector, SectorData};
use crate::ports::compliance::{ComplianceError, ComplianceResult};

/// Port trait for the Wasm plugin host.
///
/// Implementors load sector-specific Wasm plugins and delegate compliance
/// computation to them. Tests can wire a stub that returns fixed results.
/// The passthrough registry is used when no plugin is loaded for a sector.
pub trait PluginHost: Send + Sync {
    /// Returns true if a Wasm plugin is currently loaded for `sector`.
    fn has_plugin(&self, sector: &Sector) -> bool;

    /// Invoke the loaded plugin for `sector` with the given `data`.
    ///
    /// Returns `ComplianceErrorKind::UnknownSector` if no plugin is loaded.
    fn compute(
        &self,
        sector: &Sector,
        data: &SectorData,
    ) -> Result<ComplianceResult, ComplianceError>;
}
