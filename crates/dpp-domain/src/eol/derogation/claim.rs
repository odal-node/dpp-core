//! [`Derogation`] — what a destruction claim cites.

use serde::{Deserialize, Serialize};

use super::legacy::DerogationRef;
use crate::unsold_goods::DiscardReason;

/// The derogation from the ESPR Art. 25 destruction ban that a destruction relies
/// on.
///
/// # Two forms, one of them closed
///
/// [`Point`](Self::Point) is a point of the closed list of Commission Delegated
/// Regulation (EU) 2026/296 Art. 2, the vocabulary of every other derogation this
/// crate models. It is the only form this crate treats as a derogation.
///
/// [`Legacy`](Self::Legacy) is an event written when the citation was free text. It
/// is kept so that such an event still reads and writes back byte for byte, which
/// a stored event needs and a fetched one cannot do without. It is **never taken as
/// a derogation**: the text cannot be mapped to a point without inventing one. New
/// events cite a [`Point`](Self::Point).
///
/// On the wire a point is the bare string (`"damagedOrContaminated"`) and a legacy
/// citation the object it always was (`{"category": ...}`), which is what lets one
/// field read both with nothing tagging them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Derogation {
    /// A point of Del. Reg. (EU) 2026/296 Art. 2.
    Point(DiscardReason),
    /// A free-text citation written before the closed list was modelled.
    Legacy(DerogationRef),
}

impl Derogation {
    /// The point of Art. 2 this cites, or `None` for a legacy citation.
    #[must_use]
    pub fn point(&self) -> Option<DiscardReason> {
        match self {
            Self::Point(point) => Some(*point),
            Self::Legacy(_) => None,
        }
    }
}
