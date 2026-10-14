//! [`SubmissionReport`] — what became of one submitted passport.

use serde::{Deserialize, Serialize};

/// What the registry reports about a submitted passport, read by the
/// submission's correlation identifier.
///
/// One variant per thing a caller does next: keep polling, record the
/// registration, show the refusal, or keep polling while saying the registry
/// reported something unrecognised. A registration identifier exists only on
/// [`Registered`](Self::Registered) and a reason only on
/// [`Refused`](Self::Refused), so neither can be read off a submission that
/// has none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
#[non_exhaustive]
pub enum SubmissionReport {
    /// The registry has not finished checking the submission.
    Processing,
    /// The submission succeeded and the passport is registered.
    #[serde(rename_all = "camelCase")]
    Registered {
        /// The identifier the registry generated for the passport — IR (EU)
        /// 2026/1778 Art. 8(8) — and the one every later call names it by.
        registration_id: String,
    },
    /// The submission was refused.
    ///
    /// A submission is refused as a whole, so a passport can be refused for
    /// another passport's fault. `reason` is what the registry said, or says
    /// that it said nothing about this passport.
    Refused {
        /// The registry's description of the refusal.
        reason: String,
    },
    /// The registry reported a state this crate has no name for.
    ///
    /// Not a verdict. Reading it as `Refused` would discard a submission that
    /// may have succeeded, and as `Registered` would record a registration that
    /// may not exist, so a caller keeps polling and shows the state as given.
    Unrecognised {
        /// The state exactly as the registry reported it.
        state: String,
    },
}

impl SubmissionReport {
    /// Whether the registry has ruled, so polling can stop.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Registered { .. } | Self::Refused { .. })
    }
}
