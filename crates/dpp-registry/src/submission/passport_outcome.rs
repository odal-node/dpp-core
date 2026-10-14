//! [`PassportOutcome`] — what the registry ruled about one passport of a submission.

use serde::{Deserialize, Serialize};

/// What the registry ruled about one passport of a completed submission.
///
/// 👁️ **Observed** in the registry's web client on 2026-10-14: each entry of a
/// registration task's `resultPayload.report.itemDetails` carries an `outcome`
/// of `ACCEPTED` or `REJECTED`. A value outside the two deserialises as
/// [`Unrecognised`](Self::Unrecognised), with the value intact.
///
/// A submission fails as a whole, so a passport can be `REJECTED` with no
/// error of its own: it was refused with the rest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
pub enum PassportOutcome {
    /// The registry accepted this passport.
    Accepted,
    /// The registry refused this passport.
    Rejected,
    /// A value outside the two above, exactly as the registry sent it.
    Unrecognised(String),
}

impl PassportOutcome {
    /// The wire value, spelled out so a renamed variant cannot change it.
    #[must_use]
    pub fn wire_str(&self) -> &str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
            Self::Unrecognised(value) => value,
        }
    }
}

impl From<String> for PassportOutcome {
    fn from(value: String) -> Self {
        match value.as_str() {
            "ACCEPTED" => Self::Accepted,
            "REJECTED" => Self::Rejected,
            _ => Self::Unrecognised(value),
        }
    }
}

impl From<PassportOutcome> for String {
    fn from(outcome: PassportOutcome) -> Self {
        match outcome {
            PassportOutcome::Unrecognised(value) => value,
            known => known.wire_str().to_owned(),
        }
    }
}
