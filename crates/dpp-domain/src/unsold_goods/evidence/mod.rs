//! The documents the acts have an operator keep for a discard.
//!
//! - [`kind`] — [`EvidenceKind`], each document the three provisions name.
//! - [`requirement`] — [`EvidenceRequirement`], one thing a record must show, and
//!   the table of what each derogation needs.
//!
//! This crate knows what is required, never what is held: it reads no file, and
//! where documents are kept is a host's decision. A caller passes the kinds it
//! holds and is told what is missing.

pub mod kind;
pub mod requirement;

pub use kind::EvidenceKind;
pub use requirement::{EvidenceRequirement, derogation_evidence};
