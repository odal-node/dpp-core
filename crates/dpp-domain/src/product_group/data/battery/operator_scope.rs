//! [`DueDiligenceOperatorScope`] — where the operator stands against Art. 47.

use serde::{Deserialize, Serialize};

/// The operator's declaration against Regulation (EU) 2023/1542 Art. 47, first
/// paragraph, which takes operators under a net-turnover threshold out of the
/// due diligence chapter.
///
/// A declaration rather than a figure, because the threshold is tested on a
/// past financial year that a passport cannot carry and this crate cannot
/// check. Neither value names an amount, so an amendment to the threshold
/// changes no stored record. The rule that reads it is
/// [`dpp_rules::batteries::due_diligence`].
///
/// Closed, with no `#[serde(other)]`: an unrecognised value is not a third
/// answer, and must not be read as either of these two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DueDiligenceOperatorScope {
    /// The economic operator placing the battery is subject to Chapter VII.
    Subject,
    /// Art. 47, first paragraph, takes the operator out of Chapter VII.
    BelowTurnoverThreshold,
}

impl From<DueDiligenceOperatorScope> for dpp_rules::batteries::due_diligence::OperatorScope {
    fn from(scope: DueDiligenceOperatorScope) -> Self {
        match scope {
            DueDiligenceOperatorScope::Subject => Self::Subject,
            DueDiligenceOperatorScope::BelowTurnoverThreshold => Self::BelowTurnoverThreshold,
        }
    }
}
