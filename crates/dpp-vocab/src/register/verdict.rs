//! [`Verdict`] — the answer to "may we emit this IRI?", and why.

use crate::vocabulary::{Layer, Status};

/// Why an identifier may or may not be emitted.
///
/// A refusal carries its reason rather than a bare `false`. The reasons are
/// materially different — an unknown IRI is a research task, a bridge IRI is a
/// category error, and a surveyed vocabulary is a reading task — and a caller
/// that cannot tell them apart will treat all three as "add it to the
/// allowlist".
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// One of our own identifiers. Always permitted, no record needed.
    Own,
    /// Belongs to a vocabulary that has been read, at a layer we may speak.
    Permitted {
        /// The vocabulary's lookup key.
        vocabulary: String,
    },
    /// Belongs to a known vocabulary that has not been read closely enough.
    NotVerified {
        /// The vocabulary's lookup key.
        vocabulary: String,
        /// How far the record got.
        status: Status,
    },
    /// Belongs to a conformance bridge. Being measured against a model is not
    /// the same as speaking its language.
    BridgeNotSpeakable {
        /// The vocabulary's lookup key.
        vocabulary: String,
    },
    /// No vocabulary in the register claims this IRI.
    ///
    /// The most serious verdict, because it means an identifier reached a
    /// serialisation without anyone recording where it came from.
    Unknown,
}

impl Verdict {
    /// Whether the identifier may be emitted.
    #[must_use]
    pub const fn is_permitted(&self) -> bool {
        matches!(self, Self::Own | Self::Permitted { .. })
    }

    /// A one-line explanation suitable for a test failure message.
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::Own => "our own namespace".to_owned(),
            Self::Permitted { vocabulary } => format!("verified vocabulary '{vocabulary}'"),
            Self::NotVerified { vocabulary, status } => format!(
                "vocabulary '{vocabulary}' is {}, and only a read vocabulary may be emitted from",
                match status {
                    Status::Surveyed => "known only from a secondary source",
                    Status::Tracked => "evaluated and not adopted",
                    Status::Verified => "verified",
                }
            ),
            Self::BridgeNotSpeakable { vocabulary } => format!(
                "vocabulary '{vocabulary}' is a conformance bridge ({:?}); we are tested by it, we do not speak it",
                Layer::ConformanceBridge
            ),
            Self::Unknown => {
                "no vocabulary in the register claims this IRI — its origin is unrecorded"
                    .to_owned()
            }
        }
    }
}
