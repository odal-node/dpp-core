//! [`ObligationKind`] — what kind of obligation an event brings into effect.

use serde::{Deserialize, Serialize};

/// What kind of obligation an [`ObligationEvent`](super::ObligationEvent)
/// brings into effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ObligationKind {
    /// An act's passport obligation begins.
    PassportStart,
    /// A passport data point becomes owed, or stops being barred.
    ContentRequirement,
    /// A minimum share (of recycled content, say) starts to bind.
    MinimumShare,
    /// A substance restriction starts to bind, or a carve-out from one ends.
    Restriction,
}

impl From<dpp_rules::batteries::schedule::ChangeKind> for ObligationKind {
    fn from(kind: dpp_rules::batteries::schedule::ChangeKind) -> Self {
        use dpp_rules::batteries::schedule::ChangeKind;
        match kind {
            ChangeKind::ContentRequirement => Self::ContentRequirement,
            ChangeKind::MinimumShare => Self::MinimumShare,
            ChangeKind::Restriction => Self::Restriction,
        }
    }
}
