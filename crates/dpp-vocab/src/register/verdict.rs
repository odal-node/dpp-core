//! [`Verdict`] — the answer to "may we emit this IRI?", and why.

use crate::vocabulary::{Layer, Status};

/// Why an identifier may or may not be emitted.
///
/// A refusal carries its reason rather than a bare `false`. The reasons are
/// materially different — an unknown IRI is a research task, a bridge IRI is a
/// category error, and a surveyed vocabulary or an unread term is a reading task
/// — and a caller that cannot tell them apart will treat all of them as "add it
/// to the allowlist".
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// One of our own identifiers. Always permitted, no record needed.
    Own,
    /// A term somebody has read, in a vocabulary we may speak. Also the
    /// vocabulary's own namespace IRI, which names the vocabulary and makes no
    /// claim about any term in it.
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
    /// Sits in a verified vocabulary's namespace, but nobody has recorded
    /// reading this term.
    ///
    /// A namespace is a prefix, so every string beginning with it looks like a
    /// term of that vocabulary — including ones its authority never defined.
    /// Verifying the vocabulary reads the publication; it does not read the
    /// term. The remedy is a reading task, not an allowlist entry.
    TermNotRead {
        /// The vocabulary's lookup key.
        vocabulary: String,
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
            Self::TermNotRead { vocabulary } => format!(
                "vocabulary '{vocabulary}' is verified, but nobody has recorded reading this \
                 term; read its definition and list it in the record's `terms`"
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
