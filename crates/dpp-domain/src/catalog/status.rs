//! [`RegulatoryStatus`] — where a sector's DPP obligation stands in the EU
//! regulatory pipeline.

use serde::{Deserialize, Serialize};

/// Where a sector's DPP obligation stands in the EU regulatory pipeline.
///
/// This answers one question only: **does this sector's regulation create
/// binding, determinable obligations right now?** It is deliberately not the
/// same question as "is the passport mandatory yet" — see
/// [`crate::catalog::SectorDescriptor::dpp_applies_from`], which is independent
/// and does not gate determinations.
///
/// A regulation can bind years before its passport obligation begins. The
/// Batteries Regulation is the worked example: the Art. 9 mercury and cadmium
/// prohibitions have applied since 2008 (carried forward from Directive
/// 2006/66/EC) and are determinable today, while the battery passport itself is
/// only required from 2027-02-18. Battery is therefore `InForce` *and* has a
/// future `dppAppliesFrom`, and both are correct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RegulatoryStatus {
    /// The regulation creates binding obligations that can be determined
    /// **now**. Plugins may emit binding compliance determinations.
    ///
    /// A future applicability date is **not** grounds for this status — mark a
    /// sector `InForce` only when something about it is legally determinable
    /// today.
    InForce,
    /// An instrument exists or is anticipated, but nothing is bindingly
    /// determinable yet. Schemas are best-effort drafts; plugins must not
    /// assert COMPLIANT/NON_COMPLIANT — only structural validation applies.
    Provisional,
    /// Tracked, but no DPP instrument exists for this sector at all. Never
    /// determinable. Schemas, where present, are operator-defined rather than
    /// derived from law. Pairs with [`crate::catalog::Regime::None`].
    Watch,
}

impl RegulatoryStatus {
    /// Whether a sector with this status may carry a *binding* compliance
    /// determination (vs. structural validation only).
    ///
    /// `Watch` inherits the safe answer by construction rather than by a branch
    /// that could be got wrong.
    #[must_use]
    pub fn allows_determination(&self) -> bool {
        matches!(self, Self::InForce)
    }
}
