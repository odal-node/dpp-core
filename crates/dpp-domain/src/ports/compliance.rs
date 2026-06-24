//! Open-core compliance boundary — strategy and registry traits.
//!
//! This module defines the extension seam used by proprietary compliance tiers.
//!
//! The open-source (Apache-2.0) binary wires `PassthroughRegistry`, which stores
//! manufacturer-supplied values verbatim without computing any scores.
//!
//! A proprietary binary can wire its own `PremiumComplianceRegistry`
//! implementation in a separate Cargo workspace without forking this crate.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::sector::{Sector, SectorData};

// ─── Output types ─────────────────────────────────────────────────────────

/// Result of compliance calculation for a single product.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceResult {
    /// Calculated or manufacturer-supplied CO₂e score in kg.
    pub co2e_score: Option<f64>,
    /// Calculated or manufacturer-supplied repairability index (0.0–10.0).
    pub repairability_index: Option<f64>,
    /// Calculated or manufacturer-supplied recycled content percentage.
    pub recycled_content_pct: Option<f64>,
    /// Overall compliance determination.
    pub compliance_status: ComplianceStatus,
}

/// Overall compliance determination for a passport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ComplianceStatus {
    /// Manufacturer-supplied values stored verbatim — no calculation performed.
    PassthroughNoValidation,
    /// Calculated and compliant with applicable EU regulation.
    Compliant,
    /// Calculated; one or more fields fall below regulatory thresholds.
    NonCompliant,
    /// The sector's DPP obligation is not yet in force (provisional), so no
    /// binding determination is legally applicable — only structural validation
    /// was performed. See [`gate_determination`].
    NotAssessed,
    /// Sector not yet implemented by this registry.
    NotImplemented,
}

/// Enforce regulatory status on a raw determination.
///
/// A sector whose DPP obligation is **not in force** (provisional) may never
/// surface a *binding* `Compliant` / `NonCompliant` — there is no legal basis
/// for the determination, so it is downgraded to [`ComplianceStatus::NotAssessed`].
/// In-force sectors pass through unchanged, as do non-binding statuses. Callers
/// obtain `in_force` from [`crate::catalog::SectorCatalog::is_in_force`].
#[must_use]
pub fn gate_determination(in_force: bool, raw: ComplianceStatus) -> ComplianceStatus {
    if in_force {
        return raw;
    }
    match raw {
        ComplianceStatus::Compliant | ComplianceStatus::NonCompliant => {
            ComplianceStatus::NotAssessed
        }
        other => other,
    }
}

// ─── Error types ──────────────────────────────────────────────────────────

/// Error returned by a compliance strategy or registry.
#[derive(Debug, Clone)]
pub struct ComplianceError {
    pub kind: ComplianceErrorKind,
    pub message: String,
}

/// Classification of compliance calculation errors.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ComplianceErrorKind {
    /// No strategy registered for the requested sector.
    UnknownSector,
    /// Input sector data is structurally invalid for this strategy.
    InvalidInput,
    /// Internal error; should not propagate to the user.
    Internal,
}

impl fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ComplianceError {}

// ─── Traits ───────────────────────────────────────────────────────────────

/// Per-sector compliance calculation strategy.
///
/// The OSS binary ships `PassthroughBatteryStrategy` and `PassthroughTextileStrategy`.
/// Proprietary tiers implement `PremiumBatteryStrategy`, etc.
pub trait ComplianceStrategy: Send + Sync {
    /// The sector this strategy handles.
    fn sector(&self) -> Sector;

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
        sector: Sector,
        data: &SectorData,
    ) -> Result<ComplianceResult, ComplianceError>;
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provisional_downgrades_binding_determinations() {
        assert_eq!(
            gate_determination(false, ComplianceStatus::Compliant),
            ComplianceStatus::NotAssessed
        );
        assert_eq!(
            gate_determination(false, ComplianceStatus::NonCompliant),
            ComplianceStatus::NotAssessed
        );
    }

    #[test]
    fn in_force_preserves_determinations() {
        assert_eq!(
            gate_determination(true, ComplianceStatus::Compliant),
            ComplianceStatus::Compliant
        );
        assert_eq!(
            gate_determination(true, ComplianceStatus::NonCompliant),
            ComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn non_binding_statuses_pass_through_regardless() {
        for in_force in [true, false] {
            assert_eq!(
                gate_determination(in_force, ComplianceStatus::PassthroughNoValidation),
                ComplianceStatus::PassthroughNoValidation
            );
            assert_eq!(
                gate_determination(in_force, ComplianceStatus::NotAssessed),
                ComplianceStatus::NotAssessed
            );
        }
    }
}
