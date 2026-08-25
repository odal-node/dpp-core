//! [`RegulatoryStatus`] — where a sector's DPP obligation stands in the EU
//! regulatory pipeline.

use serde::{Deserialize, Serialize};

/// Where one act's obligations stand for one product group.
///
/// Held on [`InstrumentBinding`](crate::catalog::InstrumentBinding), not on the
/// product group: a group is routinely in force under one act and provisional
/// under another, so a single per-group value must either assert against an
/// unadopted act or stay silent about an adopted one.
///
/// This answers one question only: **does this act create binding, determinable
/// obligations for this product group right now?** It is deliberately not the
/// same question as "is a passport owed" — that is
/// [`PassportObligation`](crate::catalog::PassportObligation), which is
/// independent and does not gate determinations.
///
/// An act can bind years before its passport obligation begins. The Batteries
/// Regulation is the worked example: the Art. 9 mercury and cadmium
/// prohibitions have applied since 2008 (carried forward from Directive
/// 2006/66/EC) and are determinable today, while the battery passport itself is
/// only required from 2027-02-18. That binding is therefore `InForce` *and*
/// carries a future passport date, and both are correct.
///
/// The independence runs the other way too, which is what the two-field split
/// exists for: ESPR Arts. 24–25 bind today and impose **no passport at all**,
/// and an act whose passport is displaced onto another system still has live,
/// determinable duties of its own.
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
    /// Tracked, but this act imposes nothing on the product group — a watching
    /// brief rather than a duty. Never determinable. Schemas, where present, are
    /// operator-defined rather than derived from law.
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
