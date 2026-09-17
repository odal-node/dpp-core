//! The Article 9 proof of registration — the artefact that evidences the
//! registration obligation was discharged.
//!
//! # Why this is modelled before the API that fetches it
//!
//! `endpoint` records that the registry API specification is unpublished, and
//! nothing here invents a retrieval call. But Commission Implementing Regulation
//! (EU) 2026/1778 **Art. 9(2)** states the document's content directly, so the
//! shape does not depend on the specification — only getting hold of one does.
//!
//! # What makes it different from everything else in this crate
//!
//! It is the only artefact in the registration flow carrying **the Commission's
//! own qualified electronic seal and electronic time stamp**, and it binds a
//! **hash of a specific passport version** to a registration event. That is the
//! provenance chain this workspace already builds locally, terminating in an
//! external qualified anchor instead of our own signature. It is what an
//! operator hands a market surveillance authority.
//!
//! # And it is received, never produced
//!
//! Art. 9(1) puts generation on the registry, and Art. 9(4) has the Commission
//! making it available. Nothing here builds one — a proof this workspace
//! constructed would carry no Commission seal and would evidence nothing. The
//! type exists to *read* one, which is why its validation asks whether what
//! arrived carries the Art. 9(2) minimum rather than whether it is fit to send.

mod document;
#[cfg(test)]
mod tests;

pub use document::{AVAILABILITY_DAYS, ProofOfRegistration};
