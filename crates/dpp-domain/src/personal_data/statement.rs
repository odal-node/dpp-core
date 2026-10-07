//! [`PersonalDataStatement`] — the operator's statement about one marked field.

use serde::{Deserialize, Serialize};

use super::held_outside::HeldOutside;

/// What the operator states about personal data for one field its schema marks
/// as able to hold it.
///
/// Both forms make the same assertion about the field's value: it carries no
/// personal data beyond what the governing act requires the passport itself to
/// carry. They differ only in where related personal data is held. See the
/// module documentation for why that is never inside the passport.
///
/// On the wire, keyed by the field's dotted path in the passport's
/// `personalData` map:
///
/// ```json
/// { "usageHistory.negativeEvents": { "held": "nothing" } }
/// { "usageHistory.negativeEvents": { "held": "outside", "lawfulBasis": "consent", "record": "…" } }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "held")]
#[non_exhaustive]
pub enum PersonalDataStatement {
    /// No personal data related to this field of this passport is held.
    #[serde(rename = "nothing")]
    NothingHeld,
    /// Personal data related to this field is held outside the passport, in an
    /// erasable record.
    #[serde(rename = "outside")]
    HeldOutside(HeldOutside),
}
