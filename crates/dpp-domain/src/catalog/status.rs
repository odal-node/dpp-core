//! [`RegulatoryStatus`] — where a sector's DPP obligation stands in the EU
//! regulatory pipeline.

use serde::{Deserialize, Serialize};

/// Where a sector's DPP obligation stands in the EU regulatory pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RegulatoryStatus {
    /// A DPP / ecodesign obligation is legally in force, or has a firm adopted
    /// applicability date. Plugins may emit binding compliance determinations.
    InForce,
    /// On the ESPR working plan, or a delegated act is anticipated, but no DPP
    /// obligation is in force yet. Schemas are best-effort drafts; plugins must
    /// not assert COMPLIANT/NON_COMPLIANT — only structural validation applies.
    Provisional,
}

impl RegulatoryStatus {
    /// Whether a sector with this status may carry a *binding* compliance
    /// determination (vs. structural validation only).
    #[must_use]
    pub fn allows_determination(&self) -> bool {
        matches!(self, Self::InForce)
    }
}
