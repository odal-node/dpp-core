//! Open-core compliance boundary — strategy and registry traits.
//!
//! This module defines the extension seam used by proprietary compliance tiers.
//!
//! The open-source (Apache-2.0) binary wires `PassthroughRegistry`, which stores
//! manufacturer-supplied values verbatim without computing any scores.
//!
//! A proprietary binary can wire its own `PremiumComplianceRegistry`
//! implementation in a separate Cargo workspace without forking this crate.
//!
//! The value objects these traits produce — [`ComplianceResult`] and friends —
//! are domain values, not ports, and live in [`crate::domain::compliance`].
//! They are re-exported here so existing paths keep resolving.

use crate::domain::sector::SectorData;

pub use crate::domain::compliance::{
    ComplianceError, ComplianceErrorKind, ComplianceFinding, ComplianceResult, ComplianceStatus,
    gate_determination,
};

// ─── Traits ───────────────────────────────────────────────────────────────

/// Per-sector compliance calculation strategy.
///
/// The OSS binary ships `PassthroughBatteryStrategy` and `PassthroughTextileStrategy`.
/// Proprietary tiers implement `PremiumBatteryStrategy`, etc.
pub trait ComplianceStrategy: Send + Sync {
    /// The catalog key of the sector this strategy handles.
    ///
    /// A key rather than the `Sector` enum: sector identity is catalog data,
    /// and a strategy for a sector this build has no variant for is exactly the
    /// case the open sector axis exists to allow.
    fn sector_key(&self) -> &str;

    /// Compute a `ComplianceResult` from raw sector data.
    ///
    /// The passthrough implementation returns manufacturer-supplied values verbatim.
    /// A premium implementation runs calculations against EU methodology databases.
    fn compute(&self, data: &SectorData) -> Result<ComplianceResult, ComplianceError>;
}

/// Registry that dispatches to the correct `ComplianceStrategy` by sector.
///
/// The open-source default is `PassthroughRegistry`.
/// A proprietary binary can wire `PremiumComplianceRegistry` instead.
///
/// No `dpp-domain` code changes are required to swap implementations —
/// simply wire a different `Arc<dyn ComplianceRegistry>` at startup.
pub trait ComplianceRegistry: Send + Sync {
    /// Run compliance calculation for the given sector and data.
    ///
    /// Returns `ComplianceErrorKind::UnknownSector` if no strategy is registered
    /// for the requested sector.
    fn compute(
        &self,
        sector_key: &str,
        data: &SectorData,
    ) -> Result<ComplianceResult, ComplianceError>;
}
