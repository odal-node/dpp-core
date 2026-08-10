//! [`Status`] — how much of a vocabulary record rests on somebody actually
//! reading the authority's own publication.

use serde::{Deserialize, Serialize};

/// What has been established about a vocabulary.
///
/// There are deliberately only three states and no middle one. A term may be
/// emitted only from a [`Status::Verified`] vocabulary; everything else is
/// refused, including vocabularies we are confident about. Confidence is what
/// put six wrong IDTA identifiers into a published crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Status {
    /// A person read the authority's own publication and recorded what it said.
    /// The only status from which a term may be emitted.
    Verified,
    /// Evaluated and not adopted, with a recorded finding explaining why.
    /// Refused — but refused for a reason somebody can re-examine.
    Tracked,
    /// Known to exist, from a secondary source. **Nothing has been read.**
    ///
    /// The weakest state, and the easiest to mistake for knowledge: a survey
    /// can be wrong about the namespace IRI, the version, the licence and the
    /// scope simultaneously, and every one of those errors looks like a fact.
    Surveyed,
}

impl Status {
    /// Whether a term belonging to a vocabulary in this state may be emitted.
    #[must_use]
    pub const fn permits_emission(self) -> bool {
        matches!(self, Self::Verified)
    }
}
