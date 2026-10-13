//! [`ReadTerm`] — one term of a vocabulary that somebody has read.

use serde::{Deserialize, Serialize};

/// A single term, and the fact that a person has read its definition.
///
/// # Why this is a separate record from the vocabulary
///
/// A vocabulary being [`Verified`](super::Status::Verified) says its publication
/// was read, its licence established and its namespace confirmed. It says
/// nothing about any one of the terms inside it, and a namespace is open-ended:
/// `https://ref.gs1.org/voc/` is a prefix, so *every* string beginning with it
/// looks like a GS1 term, including ones GS1 never defined. Reading `gtin` does
/// not make `gs1:totallyMadeUp` true, and a gate that cannot tell the two apart
/// is checking the spelling of a prefix, not the provenance of a claim.
///
/// So the gate answers at the grain the claim is made at. A third-party IRI is
/// permitted when it is listed here, under a vocabulary we may speak.
///
/// # The IRI need not sit under `namespaceIri`
///
/// Some terms are defined by one authority and published by another, or live at
/// an address the vocabulary's namespace does not cover. A record may list such
/// a term, provided no other record claims that address; a test enforces it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ReadTerm {
    /// The absolute IRI, exactly as it would be emitted.
    pub iri: String,
    /// ISO date the definition was read. Never later than the record's
    /// `checkedOn`, which is the most recent look at anything in it.
    pub read_on: String,
    /// Where the definition was read.
    pub source: String,
    /// What the definition says, and how our use of the term fits it. A term
    /// whose fit is partial says so here, not in a comment at the call site.
    pub finding: String,
}
