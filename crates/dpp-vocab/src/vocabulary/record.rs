//! [`Vocabulary`] — one upstream vocabulary, as one file in `vocabularies/`.

use serde::{Deserialize, Serialize};

use super::{Layer, Status};

/// Everything recorded about a single upstream vocabulary.
///
/// One per file in `vocabularies/`, never two — a file that described two
/// authorities would give a reader no way to tell which one a `finding`
/// belonged to.
///
/// # Why so many fields are `Option`
///
/// `None` means **not established**, never "none" and never "not applicable".
/// An unstated licence blocks reuse rather than permitting it, and a missing
/// namespace IRI means we do not know that the authority publishes one — which
/// is a different and more interesting fact than the authority having none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Vocabulary {
    /// Stable lookup key. Equal to the source file's stem, enforced by a test.
    pub key: String,
    /// The vocabulary's own name for itself.
    pub title: String,
    /// The organisation that publishes it.
    pub authority: String,
    /// What it is for, and therefore how it may be used.
    pub layer: Layer,
    /// The prefix IRI its terms sit under.
    ///
    /// `None` where the authority publishes none — which is the case for a
    /// standards series, and for a vocabulary whose IRIs exist only as a third
    /// party's coinage. The second case is the dangerous one: an IRI minted by
    /// an intermediary looks exactly like an IRI minted by the authority.
    pub namespace_iri: Option<String>,
    /// What we would use it for. Intent, not a claim that we use it today.
    pub used_for: String,
    /// SPDX identifier or a plain description of the terms. `None` when the
    /// source did not state one.
    pub licence: Option<String>,
    /// How much of this record rests on somebody reading the authority.
    pub status: Status,
    /// ISO date of the most recent look.
    pub checked_on: String,
    /// Where that look happened. A survey page and a specification are both
    /// valid answers; conflating them is not.
    pub source: String,
    /// What was learned. The load-bearing field — a record whose `finding` is
    /// empty has recorded that a vocabulary exists and nothing else.
    pub finding: String,
    /// What would move this to the next status.
    pub next_step: String,
}

impl Vocabulary {
    /// Whether a term from this vocabulary may be emitted in a serialisation.
    ///
    /// Both conditions must hold: the vocabulary has been read, and its layer
    /// is one we may speak rather than one we are measured by.
    #[must_use]
    pub fn permits_emission(&self) -> bool {
        self.status.permits_emission() && self.layer.may_be_adopted_as_prefix()
    }

    /// Whether `iri` sits inside this vocabulary's namespace.
    ///
    /// Plain prefix comparison, and deliberately so: an IRI is compared as a
    /// string, so `http://` and `https://` forms of the same namespace are
    /// different namespaces. Several EU persistent identifiers are minted on
    /// `http`, and silently upgrading one would match an IRI nobody publishes.
    #[must_use]
    pub fn contains(&self, iri: &str) -> bool {
        self.namespace_iri
            .as_deref()
            .is_some_and(|ns| iri.starts_with(ns))
    }
}
